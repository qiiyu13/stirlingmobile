.PHONY: build-engine

build-engine:
	cd engine && cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o ../app/src/main/jniLibs build --release --lib
