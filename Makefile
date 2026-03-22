wasm:
	cargo build --target wasm32-unknown-unknown --release --features wasm --no-default-features

wasm-optimized: wasm
	yarn wasm-opt -Oz --strip-debug --strip-producers --enable-bulk-memory --enable-simd --enable-nontrapping-float-to-int \
		target/wasm32-unknown-unknown/release/tonytools.wasm \
		-o target/wasm32-unknown-unknown/release/tonytools_optimized.wasm; \
	echo "WASM binary optimized successfully!"; \
	echo "Sizes:"; \
	ls -lh target/wasm32-unknown-unknown/release/tonytools*.wasm

clean:
	rm -rf target

clean-wasm:
	rm -f target/wasm32-unknown-unknown/release/*.wasm
