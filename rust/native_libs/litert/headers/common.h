#ifndef MELIORSONUS_LITERT_COMMON_H_
#define MELIORSONUS_LITERT_COMMON_H_

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#ifdef SWIG
#define TFL_CAPI_EXPORT
#elif defined(TFL_STATIC_LIBRARY_BUILD)
#define TFL_CAPI_EXPORT
#else
#if defined(_WIN32)
#ifdef TFL_COMPILE_LIBRARY
#define TFL_CAPI_EXPORT __declspec(dllexport)
#else
#define TFL_CAPI_EXPORT
#endif
#else
#define TFL_CAPI_EXPORT __attribute__((visibility("default")))
#endif
#endif

typedef enum TfLiteStatus {
  kTfLiteOk = 0,
  kTfLiteError = 1,
  kTfLiteDelegateError = 2,
  kTfLiteApplicationError = 3,
  kTfLiteDelegateDataNotFound = 4,
  kTfLiteDelegateDataWriteError = 5,
  kTfLiteDelegateDataReadError = 6,
  kTfLiteUnresolvedOps = 7,
  kTfLiteCancelled = 8,
} TfLiteStatus;

#ifdef __cplusplus
}
#endif

#endif // MELIORSONUS_LITERT_COMMON_H_
