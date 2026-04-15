# SecureZip ProGuard Rules

# Keep JNI bridge (native methods)
-keep class com.sezip.sezip.RustBridge { *; }
-keep interface com.sezip.sezip.RustBridge$ProgressCallback { *; }

# Keep Kotlin serialization
-keepattributes *Annotation*, InnerClasses
-dontnote kotlinx.serialization.AnnotationsKt
-keepclassmembers class kotlinx.serialization.json.** { *** Companion; }
-keepclasseswithmembers class kotlinx.serialization.json.** {
    kotlinx.serialization.KSerializer serializer(...);
}
-keep,includedescriptorclasses class com.sezip.sezip.model.**$$serializer { *; }
-keepclassmembers class com.sezip.sezip.model.** {
    *** Companion;
}
-keepclasseswithmembers class com.sezip.sezip.model.** {
    kotlinx.serialization.KSerializer serializer(...);
}
