use convert_case::{Case, Casing};
use proc_macro::TokenStream as PMTokenStream;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
	FnArg, Ident, ImplItem, ItemImpl, ReturnType, Type,
	parse::{Parse, Parser},
	parse_quote,
};

#[proc_macro_attribute]
pub fn handlers(_attr: PMTokenStream, tokens: PMTokenStream) -> PMTokenStream {
	let tokens = TokenStream::from(tokens);
	let item = ItemImpl::parse
		.parse2(tokens.clone())
		.map(build_handler)
		.unwrap_or_else(|error| {
			let message = format!("{error}");
			[quote! {
				compile_error!(#message);
			}]
			.to_vec()
		});

	quote! {
		#tokens
		#(#item)*
	}
	.into()
}

struct Handler {
	ident: Ident,
	inputs: Vec<(TokenStream, Box<Type>)>,
	output: Box<Type>,
}
fn build_handler(item: ItemImpl) -> Vec<TokenStream> {
	let mut streams = Vec::new();
	let mut types: Vec<Handler> = Vec::new();
	let item_ty = item.self_ty.clone();
	let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
	for item in item.items {
		let ImplItem::Fn(item) = item else {
			continue;
		};
		let ts_name = item
			.sig
			.ident
			.to_string()
			.to_case(convert_case::Case::Camel);

		// item.sig.inputs.pop();
		let inputs = item
			.sig
			.inputs
			.into_iter()
			.filter(|x| matches!(x, FnArg::Typed(_)))
			.map(|x| match x {
				syn::FnArg::Receiver(_) => {
					unreachable!()
				}
				syn::FnArg::Typed(pat_type) => {
					let ident = pat_type.pat.to_token_stream();
					(ident, pat_type.ty)
				}
			})
			.collect();
		let output = match item.sig.output {
			ReturnType::Default => parse_quote!(()),
			ReturnType::Type(_rarrow, ty) => ty,
		};

		types.push(Handler {
			ident: Ident::new(&ts_name, item.sig.ident.span()),
			inputs,
			output,
		});
	}

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
					type_output.push_str(#name);
					type_output.push_str(": ");
					type_output.push_str(
						&<#input as ::ts_rs::TS>::ident(&config)
					);
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
			let ident = ty.ident.to_string();
			quote! {
				type_output.push_str("  ");
				type_output.push_str(#ident);
				type_output.push_str("(");
				#(#inputs)*
				type_output.push_str("): Promise<");
				type_output.push_str(&<#output as ::ts_rs::TS>::ident(&config));
				type_output.push_str(r#"> {
		return window.webkit.messageHandlers.ui.postMessage({
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
				type_output.push_str("    });\n  }\n");
			}
		})
		.collect::<Vec<_>>();
	let handlers = types.iter().map(|ty| {
		let (input_name, input_ty): (Vec<_>, Vec<_>) =
			ty.inputs.iter().map(|(a, b)| (a.to_string(), b)).unzip();

		// TokenStream::new().extend_one(Token ty.ident);
		let ident = &ty.ident;
		let ident_str = ty.ident.to_string();
		quote!(#ident_str => {
			let return_value = self.#ident (
				#(
					crate::infra::serde::de::deserialize::<#input_ty>(
						value.object_get_property(#input_name).expect("key not found")
					).expect("failed to deserialise input"),
				)*
			);
			let context = value.context().expect("how is there no context");
			crate::infra::serde::ser::serialize(&context, return_value).expect("failed to serialise return value")
		})
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
			fn ts_export(config: &Config) -> Result<(), ::ts_rs::ExportError>
			where
				Self: Sized,
			{
				#(
					<#all_exports as ::ts_rs::TS>::export(config)
						.or_else(crate::infra::rpc::filter_export_failures)?;
				)*
				return Ok(());

				#[linkme::distributed_slice(crate::infra::rpc::HANDLER_TYPE_WRITERS)]
				static HANDLER_TYPE_WRITERS: fn(&Config) = |config| {
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
	constructor(private id: object) {}

"#);
				#(#ts_types)*
				type_output.push_str("}");
				type_output
			}

			fn handle(&mut self, value: &webkit6::javascriptcore::Value) -> webkit6::javascriptcore::Value {
				let ty = value.object_get_property("__type").unwrap().to_str();
				match ty.as_str() {
					#(#handlers)*
					_ => panic!("unknown message on {}: {ty}", #item_ty_str)
				}
			}
		}
	});
	streams
}
