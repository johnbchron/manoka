
filter := "info,wgpu=error,naga=warn,manoka=debug"

run:
	RUST_LOG={{filter}} cargo run --bin manoka
