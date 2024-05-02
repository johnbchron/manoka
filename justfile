
run:
	MTL_HUD_ENABLED=1 RUST_LOG=info,manoka=debug cargo run --bin manoka

run-gl:
	RUST_LOG=info,manoka=debug WAYLAND_DISPLAY= WGPU_BACKEND=gl cargo run --bin manoka
