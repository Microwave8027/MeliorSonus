adb uninstall com.example.meliorsonus
adb.exe shell bmgr list transports
adb shell bmgr transport com.google.android.gms/.backup.BackupTransportService
adb.exe shell bmgr wipe com.google.android.gms/.backup.BackupTransportService com.example.meliorsonus