
run-gl:
	RUST_LOG=info,manoka=debug WAYLAND_DISPLAY= WGPU_BACKEND=gl cargo run --bin manoka

run:
	RUST_LOG=info,manoka=debug cargo run --bin manoka
