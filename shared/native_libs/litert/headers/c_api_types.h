#ifndef MELIORSONUS_LITERT_C_API_TYPES_H_
#define MELIORSONUS_LITERT_C_API_TYPES_H_

#include "common.h"
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum TfLiteType {
  kTfLiteNoType = 0,
  kTfLiteFloat32 = 1,
  kTfLiteInt32 = 2,
  kTfLiteUInt8 = 3,
  kTfLiteInt64 = 4,
  kTfLiteString = 5,
  kTfLiteBool = 6,
  kTfLiteInt16 = 7,
  kTfLiteComplex64 = 8,
  kTfLiteInt8 = 9,
  kTfLiteFloat16 = 10,
  kTfLiteFloat64 = 11,
  kTfLiteComplex128 = 12,
  kTfLiteUInt64 = 13,
  kTfLiteResource = 14,
  kTfLiteVariant = 15,
  kTfLiteUInt32 = 16,
  kTfLiteUInt16 = 17,
  kTfLiteInt4 = 18,
  kTfLiteBFloat16 = 19,
} TfLiteType;

typedef struct TfLiteQuantizationParams {
  float scale;
  int32_t zero_point;
} TfLiteQuantizationParams;

typedef struct TfLiteOpaqueDelegate TfLiteOpaqueDelegate;

#ifdef __cplusplus
}
#endif

#endif // MELIORSONUS_LITERT_C_API_TYPES_H_
