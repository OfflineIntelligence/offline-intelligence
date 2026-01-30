#!/bin/bash

# Build script for Java bindings

# Compile Java classes
javac -d build/classes java/com/offlineintelligence/*.java

# Create JAR file
jar cf offline-intelligence-java.jar -C build/classes .

# Build native library
cargo build --release

# Copy native library to appropriate location
cp target/release/liboffline_intelligence_java.so build/lib/