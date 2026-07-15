@ECHO OFF
SETLOCAL
SET DIRNAME=%~dp0
IF "%DIRNAME%"=="" SET DIRNAME=.
SET APP_HOME=%DIRNAME%
SET CLASSPATH=%APP_HOME%\gradle\wrapper\gradle-wrapper.jar

IF DEFINED JAVA_HOME GOTO findJavaFromJavaHome
SET JAVA_EXE=java.exe
%JAVA_EXE% -version >NUL 2>&1
IF %ERRORLEVEL% EQU 0 GOTO execute
ECHO JAVA_HOME is not set and no java command could be found.
EXIT /B 1

:findJavaFromJavaHome
SET JAVA_EXE=%JAVA_HOME%\bin\java.exe
IF EXIST "%JAVA_EXE%" GOTO execute
ECHO JAVA_HOME points to an invalid directory: %JAVA_HOME%
EXIT /B 1

:execute
"%JAVA_EXE%" -Xmx64m -Xms64m -classpath "%CLASSPATH%" org.gradle.wrapper.GradleWrapperMain %*
IF %ERRORLEVEL% NEQ 0 EXIT /B %ERRORLEVEL%
ENDLOCAL
