#ifndef MELIORSONUS_LITERT_GPU_DELEGATE_H_
#define MELIORSONUS_LITERT_GPU_DELEGATE_H_

#include "c_api_types.h"
#include "common.h"
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum TfLiteGpuInferencePreference {
  TFLITE_GPU_INFERENCE_PREFERENCE_FAST_SINGLE_ANSWER = 0,
  TFLITE_GPU_INFERENCE_PREFERENCE_SUSTAINED_SPEED = 1,
} TfLiteGpuInferencePreference;

typedef enum TfLiteGpuInferencePriority {
  TFLITE_GPU_INFERENCE_PRIORITY_AUTO = 0,
  TFLITE_GPU_INFERENCE_PRIORITY_MIN_LATENCY = 1,
  TFLITE_GPU_INFERENCE_PRIORITY_MIN_MEMORY_USAGE = 2,
  TFLITE_GPU_INFERENCE_PRIORITY_MAX_PRECISION = 3,
} TfLiteGpuInferencePriority;

typedef enum TfLiteGpuExperimentalFlags {
  TFLITE_GPU_EXPERIMENTAL_FLAGS_NONE = 0,
  TFLITE_GPU_EXPERIMENTAL_FLAGS_ENABLE_QUANT = 1 << 0,
  TFLITE_GPU_EXPERIMENTAL_FLAGS_CL_ONLY = 1 << 1,
  TFLITE_GPU_EXPERIMENTAL_FLAGS_GL_ONLY = 1 << 2,
} TfLiteGpuExperimentalFlags;

typedef struct TfLiteGpuDelegateOptionsV2 {
  int32_t is_precision_loss_allowed;
  int32_t inference_preference;
  int32_t inference_priority1;
  int32_t inference_priority2;
  int32_t inference_priority3;
  int64_t experimental_flags;
  int32_t max_delegated_partitions;
  const char* model_token;
  const char* serialization_dir;
} TfLiteGpuDelegateOptionsV2;

TFL_CAPI_EXPORT extern TfLiteGpuDelegateOptionsV2 TfLiteGpuDelegateOptionsV2Default(void);

TFL_CAPI_EXPORT extern TfLiteOpaqueDelegate*
TfLiteGpuDelegateV2Create(const TfLiteGpuDelegateOptionsV2* options);

TFL_CAPI_EXPORT extern void
TfLiteGpuDelegateV2Delete(TfLiteOpaqueDelegate* delegate);

#ifdef __cplusplus
}
#endif

#endif // MELIORSONUS_LITERT_GPU_DELEGATE_H_
