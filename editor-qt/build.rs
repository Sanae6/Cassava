use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
	CxxQtBuilder::new_qml_module(QmlModule::new("ca.sanae.Cassava").qml_file("qml/main.qml"))
		.files(["src/main.rs"])
		.qt_module("Network")
		.build();
}
