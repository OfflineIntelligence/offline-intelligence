#!/bin/bash
# Build script for Offline Intelligence Java Bindings

echo "Building Offline Intelligence Java Bindings..."

# Clean previous builds
mvn clean

# Compile the project
mvn compile

# Create JAR
mvn package

echo "Build complete! JAR file created in target/ directory"