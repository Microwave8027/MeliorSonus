package com.example.meliorsonus.db

import platform.Foundation.NSDate
import platform.Foundation.timeIntervalSince1970

actual fun getCurrentEpochMillis(): Long = (NSDate().timeIntervalSince1970 * 1000).toLong()
