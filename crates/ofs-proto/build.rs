fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_dir = std::path::Path::new("../../protos");

    let proto_files: Vec<std::path::PathBuf> = [
        "feast/types/Value.proto",
        "feast/core/Feature.proto",
        "feast/core/Entity.proto",
        "feast/core/DataFormat.proto",
        "feast/core/DataSource.proto",
        "feast/core/Aggregation.proto",
        "feast/core/Transformation.proto",
        "feast/core/FeatureViewProjection.proto",
        "feast/core/FeatureView.proto",
        "feast/core/FeatureService.proto",
        "feast/core/OnDemandFeatureView.proto",
        "feast/core/StreamFeatureView.proto",
        "feast/core/Registry.proto",
        "feast/serving/ServingService.proto",
    ]
    .iter()
    .map(|f| proto_dir.join(f))
    .collect();

    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .include_file("_includes.rs")
        .compile_protos(&proto_files, &[proto_dir])?;

    Ok(())
}
