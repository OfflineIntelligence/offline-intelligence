@echo off
REM Build script for Offline Intelligence Java Bindings (Windows)

echo Building Offline Intelligence Java Bindings...

REM Clean previous builds
call mvn clean

REM Compile the project
call mvn compile

REM Create JAR
call mvn package

echo Build complete! JAR file created in target/ directory