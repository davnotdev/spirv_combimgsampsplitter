set -e

glslc test.frag -o test.spv
glslc test_arrayed.frag -o test_arrayed.spv
glslc test_nested.frag -o test_nested.spv
glslc test_mixed.frag -o test_mixed.spv
