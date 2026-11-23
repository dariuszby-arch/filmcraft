@echo off
setlocal
cd /d "%~dp0"
echo FilmCraft 0.20.1 - wersja z HEVC Main 10 Intel QSV / libx265
where cargo >nul 2>nul
if errorlevel 1 (
 echo BRAK CARGO. Zainstaluj Rust 1.95+ ze strony https://rustup.rs i uruchom nowe CMD.
 exit /b 1
)
where ffmpeg >nul 2>nul
if errorlevel 1 (
 echo UWAGA: Nie znaleziono FFmpeg w PATH.
 echo Po kompilacji umiesc ffmpeg.exe obok target\release\filmcraft.exe
)
cargo build --release -p filmcraft
if errorlevel 1 (
 echo KOMPILACJA NIE POWIODLA SIE. Sprawdz komunikaty Cargo powyzej.
 exit /b 1
)
echo.
echo Kompilacja zakonczona. Plik: "%CD%\target\release\filmcraft.exe"
echo PAMIETAJ: filmcraft.exe wymaga ffmpeg.exe w PATH lub obok EXE.
endlocal
