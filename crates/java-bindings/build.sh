#!/bin/bash

javac -d build/classes java/com/offlineintelligence/*.java

jar cf offline-intelligence-java.jar -C build/classes .

cargo build --release

cp target/release/liboffline_intelligence_java.so build/lib/
