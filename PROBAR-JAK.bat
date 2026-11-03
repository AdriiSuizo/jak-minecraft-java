@echo off
setlocal
set "JAVA_HOME=%LOCALAPPDATA%\Packages\Microsoft.4297127D64EC6_8wekyb3d8bbwe\LocalCache\Local\runtime\java-runtime-epsilon\windows-x64\java-runtime-epsilon"
if not exist "%JAVA_HOME%\bin\java.exe" (
 echo No se encuentra Java 25 en la ruta del launcher de Minecraft.
 echo Consulta README.md antes de cambiar la configuracion de Java.
 pause
 exit /b 1
)
pushd "%~dp0"
echo Abriendo Minecraft Fabric con Jak, en el perfil de desarrollo separado...
echo La primera carga de modelos y sonidos puede tardar.
call gradlew.bat runClient --no-daemon --console=plain
set "JAK_EXIT=%ERRORLEVEL%"
popd
if not "%JAK_EXIT%"=="0" (
 echo.
 echo Minecraft no ha arrancado correctamente. Conserva este mensaje.
 pause
)
exit /b %JAK_EXIT%
