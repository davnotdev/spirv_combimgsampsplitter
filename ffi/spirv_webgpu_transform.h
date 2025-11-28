#ifndef SPIRV_WEBGPU_TRANSFORM_H
#define SPIRV_WEBGPU_TRANSFORM_H

#include <stdint.h>

extern "C" {

typedef void* TransformCorrectionMap;

#define SPIRV_WEBGPU_TRANFORM_CORRECTION_MAP_NULL NULL

void spirv_webgpu_transform_combimgsampsplitter_alloc(uint32_t* in_spv, uint32_t in_count, uint32_t** out_spv, uint32_t* out_count, TransformCorrectionMap* correction_map);
void spirv_webgpu_transform_combimgsampsplitter_free(uint32_t* out_spv);
void spirv_webgpu_transform_drefsplitter_alloc(uint32_t* in_spv, uint32_t in_count, uint32_t** out_spv, uint32_t* out_count, TransformCorrectionMap* correction_map);
void spirv_webgpu_transform_drefsplitter_free(uint32_t* out_spv);

void spirv_webgpu_transform_correction_map_free(TransformCorrectionMap correction_map);

// TODO: Indexing set binding with verbose enum response

}

#endif
