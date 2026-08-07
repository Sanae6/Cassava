fn main() {
	println!("cargo::rerun-if-changed=resources/Cassava.ui");
	glib_build_tools::compile_resources(
		&["resources/"],
		"resources/resources.gresource.xml",
		"resources.gresource",
	);
}
