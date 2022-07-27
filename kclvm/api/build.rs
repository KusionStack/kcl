fn main() {
    protobuf_codegen::Codegen::new()
        .protoc()
        .protoc_path(&protoc_bin_vendored::protoc_bin_path().unwrap())
        .out_dir("src/model")
        .include(".")
        .inputs(&["./gpyrpc.proto"])
        .run()
        .expect("Running protoc failed.");
}
