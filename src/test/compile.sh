set -e

glslc splitcombined/test.frag -o splitcombined/test.spv
glslc splitcombined/test_arrayed.frag -o splitcombined/test_arrayed.spv
glslc splitcombined/test_nested.frag -o splitcombined/test_nested.spv
glslc splitcombined/test_mixed.frag -o splitcombined/test_mixed.spv

glslc splitdref/test.frag -o splitdref/test.spv
