#ifndef MELIORSONUS_LITERT_COREML_DELEGATE_H_
#define MELIORSONUS_LITERT_COREML_DELEGATE_H_

#include "c_api_types.h"
#include "common.h"
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum TfLiteCoreMlDelegateEnabledDevices {
  TfLiteCoreMlDelegateDevicesAll = 0,
  TfLiteCoreMlDelegateDevicesCpuOnly = 1,
  TfLiteCoreMlDelegateDevicesNeuralEngineOnly = 2,
} TfLiteCoreMlDelegateEnabledDevices;

typedef struct TfLiteCoreMlDelegateOptions {
  TfLiteCoreMlDelegateEnabledDevices enabled_devices;
  int32_t coreml_version;
  int32_t max_delegated_partitions;
  int32_t min_nodes_per_partition;
} TfLiteCoreMlDelegateOptions;

TFL_CAPI_EXPORT extern TfLiteOpaqueDelegate*
TfLiteCoreMlDelegateCreate(const TfLiteCoreMlDelegateOptions* options);

TFL_CAPI_EXPORT extern void
TfLiteCoreMlDelegateDelete(TfLiteOpaqueDelegate* delegate);

#ifdef __cplusplus
}
#endif

#endif // MELIORSONUS_LITERT_COREML_DELEGATE_H_
