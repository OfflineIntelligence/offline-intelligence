#!/usr/bin/env python3
"""
Build and publish script for Offline Intelligence Python package
"""

import os
import subprocess
import sys
import shutil
from pathlib import Path

def run_command(cmd, cwd=None):
    """Run a command and return the result"""
    print(f"Running: {' '.join(cmd)}")
    result = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"Error: {result.stderr}")
        return False
    print(result.stdout)
    return True

def main():
    # Get the current directory (should be python-bindings)
    current_dir = Path.cwd()
    project_root = current_dir.parent.parent
    
    print(f"Building Offline Intelligence Python package")
    print(f"Current directory: {current_dir}")
    print(f"Project root: {project_root}")
    
    # Step 1: Build the Rust extension
    print("\n1. Building Rust extension...")
    if not run_command(["cargo", "build", "--release"], cwd=project_root):
        print("Failed to build Rust extension")
        return False
    
    # Step 2: Copy the built extension to the Python package
    print("\n2. Copying extension module...")
    target_dir = project_root / "target" / "release"
    
    # Determine the extension file based on platform
    if sys.platform == "win32":
        ext_file = "offline_intelligence_py.dll"
    elif sys.platform == "darwin":
        ext_file = "liboffline_intelligence_py.dylib"
    else:  # Linux and others
        ext_file = "liboffline_intelligence_py.so"
    
    source_path = target_dir / ext_file
    dest_dir = current_dir / "offline_intelligence_py"
    dest_path = dest_dir / ext_file
    
    if source_path.exists():
        # Create destination directory if it doesn't exist
        dest_dir.mkdir(exist_ok=True)
        
        # Copy the file
        shutil.copy2(source_path, dest_path)
        print(f"Copied {source_path} to {dest_path}")
    else:
        print(f"Extension file not found at {source_path}")
        return False
    
    # Step 3: Create distributions
    print("\n3. Creating source distribution...")
    if not run_command([sys.executable, "setup.py", "sdist"]):
        print("Failed to create source distribution")
        return False
    
    print("\n4. Creating wheel distribution...")
    if not run_command([sys.executable, "setup.py", "bdist_wheel"]):
        print("Failed to create wheel distribution")
        return False
    
    # Step 4: Show what was created
    dist_dir = current_dir / "dist"
    if dist_dir.exists():
        print(f"\nCreated distributions in {dist_dir}:")
        for file in dist_dir.iterdir():
            if file.is_file():
                print(f"  - {file.name}")
    
    print("\nBuild completed successfully!")
    print("\nTo publish to PyPI:")
    print("  twine upload dist/*")
    print("\nTo publish to Test PyPI first:")
    print("  twine upload --repository testpypi dist/*")
    
    return True

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)