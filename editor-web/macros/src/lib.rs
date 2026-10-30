use convert_case::{Case, Casing};
use proc_macro::TokenStream as PMTokenStream;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote, quote_spanned};
use syn::{
	Error, FnArg, Ident, ImplItem, ItemImpl, ReturnType, Token, Type,
	parse::{Parse, ParseStream, Parser},
	parse_quote,
	punctuated::Punctuated,
	spanned::Spanned,
	token::{Async, Semi},
};

#[proc_macro_attribute]
pub fn handlers(_attr: PMTokenStream, tokens: PMTokenStream) -> PMTokenStream {
	let tokens = TokenStream::from(tokens);
	let item = ItemImpl::parse
		.parse2(tokens.clone())
		.map(|item| build_handler(item, tokens))
		.unwrap_or_else(|error| {
			let message = format!("{error}");
			[quote! {
				compile_error!(#message);
			}]
			.to_vec()
		});

	quote! {
		#(#item)*
	}
	.into()
}

struct Handler {
	ident: Ident,
	asyncness: Option<Async>,
	requested_context: bool,
	report_progress: Option<Vec<(Ident, Box<Type>)>>,
	inputs: Vec<(TokenStream, Box<Type>)>,
	output: Box<Type>,
}
fn build_handler(mut item: ItemImpl, item_tokens: TokenStream) -> Vec<TokenStream> {
	let mut streams = Vec::new();
	let mut types: Vec<Handler> = Vec::new();
	let mut mutated_item = false;
	let item_ty = item.self_ty.clone();
	let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
	for item in &mut item.items {
		let ImplItem::Fn(item) = item else {
			continue;
		};
		let ts_name = item.sig.ident.to_string();
		let report_progress = if let Some((index, attr)) = item
			.attrs
			.iter()
			.enumerate()
			.find(|(_, attr)| attr.path().is_ident("report_progress"))
		{
			mutated_item = true;

			let parse_report_progress = |input: ParseStream| {
				let mut args = Vec::new();
				loop {
					let ident = input.parse()?;
					let _: Token![:] = input.parse()?;
					let ty = input.parse()?;
					args.push((ident, ty));
					if !input.peek(Token![,]) {
						break;
					}
					let _: Token![,] = input.parse()?;
					if input.is_empty() {
						break;
					}
				}
				Ok(args)
			};
			let args = match attr.parse_args_with(parse_report_progress) {
				Ok(value) => Some(value),
				Err(error) => {
					let error_message = format!("failed to parse report_progress attribute: {error}");
					streams.push(quote_spanned! {attr.span()=>compile_error!(#error_message);});
					None
				}
			};
			item.attrs.remove(index);
			args
		} else {
			None
		};

		let requested_context = item.sig.inputs.iter().any(|x| match x {
			FnArg::Receiver(_) => false,
			syn::FnArg::Typed(pat_type) => pat_type.pat.to_token_stream().to_string() == "call_context",
		});
		let asyncness = item.sig.asyncness;
		if let Some(asyncness) = &asyncness {
			let has_receiver = item.sig.inputs.iter().any(|x| match x {
				FnArg::Receiver(_) => true,
				syn::FnArg::Typed(_) => false,
			});
			if has_receiver {
				streams.push(quote_spanned! {asyncness.span=>
					compile_error!("async functions cannot have a receiver (e.g. &mut self)");
				});
			}
			if !requested_context {
				streams.push(quote_spanned! {asyncness.span=>
					compile_error!("async functions must have a calling context argument (call_context: Context<Self>)");
				});
			}
		} else if report_progress.is_some() {
			streams.push(quote! {
				compile_error!("progress can only be reported on async functions");
			});
		}
		let inputs = item
			.sig
			.inputs
			.iter()
			.filter(|x| match x {
				FnArg::Receiver(_) => false,
				syn::FnArg::Typed(pat_type) => pat_type.pat.to_token_stream().to_string() != "call_context",
			})
			.map(|x| match x {
				syn::FnArg::Receiver(_) => {
					unreachable!()
				}
				syn::FnArg::Typed(pat_type) => {
					let ident = pat_type.pat.to_token_stream();
					(ident, pat_type.ty.clone())
				}
			})
			.collect();
		let output = match &item.sig.output {
			ReturnType::Default => parse_quote!(()),
			ReturnType::Type(_rarrow, ty) => ty.clone(),
		};

		types.push(Handler {
			ident: Ident::new(&ts_name, item.sig.ident.span()),
			asyncness,
			requested_context,
			report_progress,
			inputs,
			output,
		});
	}
	streams.push(if mutated_item {
		item.to_token_stream()
	} else {
		item_tokens
	});

	let all_dependencies = types
		.iter()
		.map(|ty| {
			let mut stream = ty
				.inputs
				.iter()
				.map(|inp| inp.1.to_token_stream())
				.collect::<Vec<_>>();
			stream.push(ty.output.to_token_stream());
			stream
		})
		.flatten();
	let all_exports = all_dependencies.clone();
	let ts_types = types
		.iter()
		.map(|ty| {
			let inputs = ty.inputs.iter().map(|input| {
				let name = &input.0.to_string().to_case(Case::Camel);
				let input = &input.1;
				quote!(
					type_output.push_str("    ");
					type_output.push_str(#name);
					type_output.push_str(": ");
					type_output.push_str(
						&<#input as ::ts_rs::TS>::ident(&config)
					);
					type_output.push_str(",\n");
				)
			});
			let input_keys = ty
				.inputs
				.iter()
				.map(|input| input.0.to_string().to_case(Case::Camel));
			// format!(
			// "  {}({}): <{} as ts_rs::TS>::ident()",
			// ,
			// inputs,
			// ty.output.to_token_stream().to_string()
			// ));
			let output = &ty.output;
			let ident_camel = ty.ident.to_string().to_case(Case::Camel);
			let ident = ty.ident.to_string();
			let async_keyword = ty
				.asyncness
				.is_some()
				.then(|| quote!(type_output.push_str("async ");));
			let await_keywords = ty
				.asyncness
				.is_some()
				.then(|| quote!(type_output.push_str("await ");));
			let progress_input = ty.report_progress.as_ref().map(|args| {
				let args = args.iter().map(|(ident, ty)| {
					let ident = ident.to_string().to_case(Case::Camel);
					quote! {
						type_output.push_str("      ");
						type_output.push_str(#ident);
						type_output.push_str(": ");
						type_output.push_str(&<#ty as ::ts_rs::TS>::ident(&config));
						type_output.push_str(",\n");
					}
				});
				quote!(
					type_output.push_str("    progressCallback: (\n");
					#(#args)*
					type_output.push_str("    ) => void,\n");
				)
			});
			let progress_register = ty.report_progress.is_some().then(|| {
				quote! {
					type_output.push_str("    const [
			__progressUuid,
			__progressUnsubscriber
		] = __crescentEventManager.registerProgressReporter(progressCallback);\n");	
				}
			});
			let progress_unsubscribe = ty
				.report_progress
				.is_some()
				.then(|| quote!(type_output.push_str(".finally(__progressUnsubscriber)");));
			let progress_key = ty
				.report_progress
				.is_some()
				.then(|| quote!(type_output.push_str("      __progressUuid,\n");));
			quote! {
				type_output.push_str("  ");
				#async_keyword
				type_output.push_str(#ident_camel);
				type_output.push_str("(\n");
				#(#inputs)*
				#progress_input
				type_output.push_str("  ): Promise<");
				type_output.push_str(&<#output as ::ts_rs::TS>::ident(&config));
				type_output.push_str("> {\n");
				#progress_register
				type_output.push_str("    return ");
				#await_keywords
				type_output.push_str(r#"window.webkit.messageHandlers.ui.postMessage({
			__mty: "rpc",
			__id: this.id,
		  __type: ""#);
				type_output.push_str(#ident);
				type_output.push_str("\", \n");
				#(
					type_output.push_str("      ");
					type_output.push_str(#input_keys);
					type_output.push_str(", \n");
				)*
				#progress_key
				type_output.push_str("    })");
				#progress_unsubscribe
				type_output.push_str(";\n  }\n");
			}
		})
		.collect::<Vec<_>>();
	let handlers = types.iter().map(|ty| {
		let (input_name, input_ty): (Vec<_>, Vec<_>) = ty
			.inputs
			.iter()
			.map(|(a, b)| (a.to_string().to_case(Case::Camel), b))
			.unzip();

		let use_context = ty.requested_context.then(|| quote!(call_context,));
		// TokenStream::new().extend_one(Token ty.ident);
		let ident = &ty.ident;
		let ident_str = ty.ident.to_string();
		if ty.asyncness.is_none() {
			quote!(#ident_str => {
				let return_value = self.#ident (
					#use_context
					#(
						crate::infra::serde::de::deserialize::<#input_ty>(
							value.object_get_property(#input_name).expect("key not found")
						).expect("failed to deserialise input"),
					)*
				);
				let context = value.context().expect("how is there no context");
				Ok(Some(crate::infra::serde::ser::serialize(&context, return_value)
					.expect("failed to serialise return value")))
			})
		} else {
			let input_var = ty
				.inputs
				.iter()
				.map(|ty| Ident::new(&format!("arg_{}", ty.0.to_string()), Span::call_site()))
				.collect::<Vec<_>>();
			quote!(#ident_str => {
				let context = value.context().unwrap();
				#(
						let #input_var = crate::infra::serde::de::deserialize::<#input_ty>(
							value.object_get_property(#input_name).expect("key not found")
						).expect("failed to deserialise input");
					)*
				let future = webkit6::glib::MainContext::default().spawn_local(async move {
					let call_context = unsafe { call_context.downcast_unchecked::<Self>() };
					let return_value = Self::#ident(call_context, #(#input_var,)*).await;

					reply.return_value(&crate::infra::serde::ser::serialize(&context, return_value)
						.expect("failed to serialise return value"));
				});
				Ok(None)
			})
		}
	});

	let item_ty_str = item_ty.to_token_stream().to_string();
	let path = format!("{item_ty_str}.ts");
	streams.push(quote! {
		impl #impl_generics crate::infra::rpc::ObjectHandler for #item_ty #ty_generics #where_clause {
			fn ts_dependencies(config: &::ts_rs::Config) -> Vec<::ts_rs::Dependency> {
				let mut dependencies: Vec<::ts_rs::Dependency> = Vec::new();
				#(
					if let Some(dep) = ::ts_rs::Dependency::from_ty::<#all_dependencies>(config) {
						dependencies.push(dep);
					};
					dependencies.extend(<#all_dependencies as ::ts_rs::TS>::dependencies(config));
				)*
				dependencies
			}

			fn ts_export(config: &::ts_rs::Config) -> Result<(), ::ts_rs::ExportError>
			where
				Self: Sized,
			{
				#(
					<#all_exports as ::ts_rs::TS>::export_all(config)
						.or_else(crate::infra::rpc::filter_export_failures)?;
				)*
				return Ok(());

				#[linkme::distributed_slice(crate::infra::rpc::HANDLER_TYPE_WRITERS)]
				static HANDLER_TYPE_WRITERS: fn(&::ts_rs::Config) = |config| {
					#item_ty::ts_export(&config).expect("failed to export dependencies");
					let text = #item_ty::ts_types(&config);
					std::fs::write(config.out_dir().join(&format!("{}.ts", #item_ty_str)), text)
						.expect("failed to write type file");
				};
			}
			fn ts_types(config: &::ts_rs::Config) -> String {
				let mut type_output = String::new();

				crate::infra::rpc::generate_imports(
					config,
					#path,
					Self::ts_dependencies(config),
					&mut type_output
				).expect("failed to generate imports :(");

				type_output.push_str("export class ");
				type_output.push_str(#item_ty_str);
				type_output.push_str(r#" {
	constructor(private id: object) { }

"#);
				#(#ts_types)*
				type_output.push_str("}");
				type_output
			}

			fn handle(
				&mut self,
				call_context: crate::infra::rpc::Context<dyn crate::infra::rpc::ObjectHandler>,
				reply: webkit6::ScriptMessageReply,
				value: &webkit6::javascriptcore::Value
			) -> Result<Option<webkit6::javascriptcore::Value>, crate::infra::rpc::HandlerError> {
				let ty = value.object_get_property("__type").unwrap().to_str();
				match ty.as_str() {
					#(#handlers)*
					_ => Err(crate::infra::rpc::HandlerError {
						type_string: ty.as_str().to_owned(),
						handler_name: #item_ty_str,
					})
				}
			}
		}
	});
	streams
}
