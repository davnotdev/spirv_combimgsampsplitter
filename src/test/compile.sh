set -e

glslc splitcombined/test.frag -o splitcombined/test.spv
glslc splitcombined/test_arrayed.frag -o splitcombined/test_arrayed.spv
glslc splitcombined/test_nested.frag -o splitcombined/test_nested.spv
glslc splitcombined/test_mixed.frag -o splitcombined/test_mixed.spv

glslc splitdref/test.frag -o splitdref/test.spv
glslc splitdref/test_nested.frag -o splitdref/test_nested.spv
glslc splitdref/test_nested2.frag -o splitdref/test_nested2.spv
spirv-as splitdref/test_wrong_type_image.spvasm -o splitdref/test_wrong_type_image.spv
