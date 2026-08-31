#ifndef MELIORSONUS_LITERT_XNNPACK_DELEGATE_H_
#define MELIORSONUS_LITERT_XNNPACK_DELEGATE_H_

#include "c_api_types.h"
#include "common.h"
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct TfLiteXNNPackDelegateOptions {
  int32_t num_threads;
  uint32_t flags;
} TfLiteXNNPackDelegateOptions;

TFL_CAPI_EXPORT extern TfLiteXNNPackDelegateOptions TfLiteXNNPackDelegateOptionsDefault(void);

TFL_CAPI_EXPORT extern TfLiteOpaqueDelegate*
TfLiteXNNPackDelegateCreate(const TfLiteXNNPackDelegateOptions* options);

TFL_CAPI_EXPORT extern void
TfLiteXNNPackDelegateDelete(TfLiteOpaqueDelegate* delegate);

#ifdef __cplusplus
}
#endif

#endif // MELIORSONUS_LITERT_XNNPACK_DELEGATE_H_
