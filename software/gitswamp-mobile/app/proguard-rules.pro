-dontwarn org.eclipse.jgit.**
-keepattributes Signature,InnerClasses,EnclosingMethod

# JGit discovers several commands, transports, builders, and optional providers
# through reflection. Keeping the complete runtime prevents R8 from renaming or
# removing constructors that are requested by name at runtime.
-keep class org.eclipse.jgit.** { *; }
-keep class org.apache.sshd.** { *; }
-keep class org.slf4j.** { *; }
