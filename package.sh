#!/bin/bash

# Config
BIN_NAME=dzd
BUILD_PATH=../../target/release/$BIN_NAME
DEST_DIR=dist

# Step 1: Build
cargo build --release

# Step 2: Prepare folder
rm -rf $DEST_DIR
mkdir -p $DEST_DIR/lib

# Step 3: Copy binary
cp "$BUILD_PATH" "$DEST_DIR/"

# Step 4: Copy shared libs
ldd "$BUILD_PATH" | grep "=> /" | awk '{print $3}' | while read lib; do
    cp "$lib" "$DEST_DIR/lib/"
done

# Step 5: Create launcher
cat > "$DEST_DIR/run.sh" <<EOF
#!/bin/bash
DIR="\$(cd "\$(dirname "\$0")" && pwd)"
LD_LIBRARY_PATH="\$DIR/lib" "\$DIR/$BIN_NAME" "\$@"
EOF

chmod +x "$DEST_DIR/run.sh"

echo "Packaged to $DEST_DIR/"
