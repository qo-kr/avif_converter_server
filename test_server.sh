#!/bin/bash
set -e

# Cleanup function to kill background processes
cleanup() {
    echo "Stopping servers..."
    if [ -n "$PID_PYTHON" ]; then kill $PID_PYTHON 2>/dev/null || true; fi
    if [ -n "$PID_APP" ]; then kill $PID_APP 2>/dev/null || true; fi
}
trap cleanup EXIT

echo "Building project..."
cargo build --release

echo "Starting Image Server (Port 8081)..."
python3 -m http.server 8081 > /dev/null 2>&1 &
PID_PYTHON=$!

echo "Starting App Server (Port 8080)..."
./target/release/oq-imgcomp > /dev/null 2>&1 &
PID_APP=$!

echo "Waiting for servers to initialize (5s)..."
sleep 5

echo "Running Integration Tests..."

# Helper function for checking result
check_file() {
    if [ -f "$1" ]; then
        echo "✅ Success: $1 created"
    else
        echo "❌ Failed: $1 not created"
        exit 1
    fi
}

# 1. Basic Resize (Width & Height)
echo "Test 1: Basic Resize (300x200)"
curl -s "http://localhost:8080/convert?url=http://localhost:8081/original.jpg&width=300&height=200" -o integration_test_basic.avif
check_file "integration_test_basic.avif"

# 2. Width only
echo "Test 2: Width only (Width=500)"
curl -s "http://localhost:8080/convert?url=http://localhost:8081/original.jpg&width=500" -o integration_test_width.avif
check_file "integration_test_width.avif"

# 3. Height only
echo "Test 3: Height only (Height=500)"
curl -s "http://localhost:8080/convert?url=http://localhost:8081/original.jpg&height=500" -o integration_test_height.avif
check_file "integration_test_height.avif"

# 4. Cover (Crop)
echo "Test 4: Fit Cover (300x300 Crop)"
curl -s "http://localhost:8080/convert?url=http://localhost:8081/original.jpg&width=300&height=300&fit=cover" -o integration_test_cover.avif
check_file "integration_test_cover.avif"

# 5. Fill (Stretch)
echo "Test 5: Fit Fill (300x300 Stretch)"
curl -s "http://localhost:8080/convert?url=http://localhost:8081/original.jpg&width=300&height=300&fit=fill" -o integration_test_fill.avif
check_file "integration_test_fill.avif"

# 6. Contain with White Padding
echo "Test 6: Contain with White Padding"
curl -s "http://localhost:8080/convert?url=http://localhost:8081/original.jpg&width=1280&height=960&fit=contain&bg=white" -o integration_test_white.avif
check_file "integration_test_white.avif"

# 7. Contain with Pattern Padding
echo "Test 7: Contain with Pattern Padding"
if [ -f "pattern_checker.png" ]; then
    curl -s "http://localhost:8080/convert?url=http://localhost:8081/original.jpg&width=1280&height=960&fit=contain&pattern=http://localhost:8081/pattern_checker.png" -o integration_test_pattern.avif
    check_file "integration_test_pattern.avif"
else
    echo "⚠️ Skipping Pattern Test: pattern_checker.png not found"
fi

# 8. BBox Crop (Normalized)
echo "Test 8: BBox Crop (Normalized)"
curl -s "http://localhost:8080/convert?url=http://localhost:8081/original.jpg&bbox=0,0.0992,0.5291,0.7440&bbox_unit=norm&width=800&height=600&fit=contain&bg=white" -o integration_test_bbox_norm.avif
check_file "integration_test_bbox_norm.avif"

# 9. Force JPEG Output
echo "Test 9: Force JPEG Output"
curl -s "http://localhost:8080/convert?url=http://localhost:8081/original.jpg&width=640&height=480&ext=jpg" -o integration_test_ext.jpg
check_file "integration_test_ext.jpg"

# 10. Force PNG Output
echo "Test 10: Force PNG Output"
curl -s "http://localhost:8080/convert?url=http://localhost:8081/original.jpg&width=640&height=480&ext=png" -o integration_test_ext.png
check_file "integration_test_ext.png"

# 11. Accept Header PNG
echo "Test 11: Accept Header PNG"
curl -s -H "Accept: image/png" "http://localhost:8080/convert?url=http://localhost:8081/original.jpg&width=640&height=480" -o integration_test_accept_png.png
check_file "integration_test_accept_png.png"

echo "All integration tests completed successfully!"
