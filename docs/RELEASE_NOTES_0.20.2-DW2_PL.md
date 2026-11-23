# FilmCraft 0.20.2-DW2 — HEVC Main 10 / precyzyjna ścieżka eksportu SDR

**Niezależne wydanie forka** `dariuszby-arch/filmcraft`, rozwijające stabilną wersję DW1. W DW2 poprawiono przekazywanie obrazu 10-bitowego do FFmpeg i dodano testy. DW1 pozostaje bez zmian. Bazowa aplikacja może nadal wyświetlać numer 0.20.1; `0.20.2-DW2` to oznaczenie wydania tego forka.

## Najważniejsza zmiana w DW2

- **HEVC Main 10 SDR**: po renderowaniu i efektach klatka pozostaje w liniowym formacie **RGBA float32** zamiast być przedwcześnie kwantyzowana do RGBA8. Przed wysłaniem do FFmpeg przekształcamy ją do **RGB48 LE (16 bitów na kanał)**, a dalej do `yuv420p10le` (`libx265`) lub `p010le` (Intel Quick Sync).
- **HEVC Main (8-bit)** i inne tryby eksportu zachowują dotychczasową ścieżkę DW1.
- Zachowana zgodność eksportu SDR BT.709 w MP4 (`hvc1`); dźwięk AAC jest dołączany bez ponownego stratnego kodowania.
- Nadal dostępny jest wybór Intel Quick Sync (jeśli sterownik i FFmpeg pozwalają) albo CPU `libx265`.
- Nowe testy jednostkowe sprawdzają zachowanie subtelnych poziomów jasności, dokładne kodowanie czerni i bieli oraz ogranicznik poziomów przed końcową kwantyzacją.

## Wersje do pobrania

| System | Plik |
| --- | --- |
| Windows x64 | `FilmCraft-0.20.2-DW2-Windows-x64.zip` |
| Linux x86_64 (MX Linux), AppImage | `FilmCraft-0.20.2-DW2-Linux-x86_64.AppImage` |
| Linux x86_64, archiwum przenośne | `FilmCraft-0.20.2-DW2-Linux-x86_64.tar.gz` |

W wydaniu znajdują się także: `OPIS_WYDANIA_PL.md`, `WYNIKI_TESTOW_PL.md` i `SHA256SUMS.txt`.

## Nasze testy 8-bit vs 10-bit

**Test 1 (1920 × 1080, 26,794 s, x265, CBR zadany 20 Mb/s):**

| Parametr | HEVC 8-bit | HEVC 10-bit |
| --- | ---: | ---: |
| Średni bitrate wideo | 12,60 Mb/s | 4,15 Mb/s |
| Rozmiar pliku | 43,27 MB | 14,98 MB |
| Profil | Main | Main 10 |
| Format pikseli | yuv420p | yuv420p10le |

Plik 10-bitowy był o około **65,4% mniejszy**, ale to **nie dowodzi 3-krotnie większej efektywności HEVC Main 10**: oba strumienie miały znacznie różne rzeczywiste bitrate mimo zadanych 20 Mb/s. MediaInfo potwierdził `libx265`, `rc=cbr`, `bitrate=20000`, `vbv-maxrate=20000`, `vbv-bufsize=40000` oraz **`no-strict-cbr`**. Użytkownik nie zauważył wyraźnego pogorszenia jakości. SSIM dwóch PNG (eksport kontra eksport, nie oryginał) wyniósł **0,98356**.

**Test 2 (3840 × 2160, 23,976 fps, 70,40 s, CBR zadany 20 Mb/s):** Rozmiary plików 8-bit i 10-bit były według użytkownika **niemal identyczne**. Dla trzech przesłanych PNG o rozdzielczości 2048 × 1152: SSIM 10-bit/original **0,95975**, SSIM 8-bit/original **0,96542**. Metryki obejmują tylko jedną klatkę i przeskalowane, 8-bitowe PNG; nie pozwalają rozstrzygnąć jakości całych plików HEVC ani rzeczywistej przewagi głębi 10-bit.

Pełne dane i ograniczenia interpretacji: **`WYNIKI_TESTOW_PL.md`**.

## Instalacja i uruchomienie

**Windows:** rozpakuj ZIP i uruchom `filmcraft.exe`. FFmpeg nie jest dołączony: umieść działający **`ffmpeg.exe`** (z `libx265` lub `hevc_qsv`) obok programu, dodaj go do PATH albo użyj zmiennej `FILMCRAFT_FFMPEG`.

**MX Linux:** `sudo apt install ffmpeg`, następnie `chmod +x FilmCraft-0.20.2-DW2-Linux-x86_64.AppImage` i `./FilmCraft-0.20.2-DW2-Linux-x86_64.AppImage`. Jeśli wymaga tego środowisko: `APPIMAGE_EXTRACT_AND_RUN=1 ./FilmCraft-0.20.2-DW2-Linux-x86_64.AppImage`.

**Windows EXE pod Wine:** wymaga **windowsowego `ffmpeg.exe`**; na MX Linux może działać z `WGPU_BACKEND=vulkan wine ./filmcraft.exe`. Tryb sprzętowy Intel QSV pod Wine nie jest gwarantowany.

## Ograniczenia i dalsze prace

- Ominięcie RGBA8 dotyczy **końcowej ścieżki HEVC Main 10**. Nie oznacza, że każdy dekoder, filtr, nakładka, źródło lub monitor działa od początku do końca z pełną precyzją 10-bit.
- Eksport SDR używa aktualnej funkcji przenoszenia charakterystyki obrazu FilmCraft, podobnej do sRGB, i sygnalizuje BT.709; precyzyjna zgodność transferu SDR wymaga osobnego przeglądu kolorystycznego.
- `libx265` używa w tych testach **nie-ścisłego CBR** (`no-strict-cbr`): wybrany bitrate 20 Mb/s nie gwarantuje takiej średniej przepływności wynikowej.
- Nie wykonano dotąd pełnego testu VMAF całych materiałów ani walidacji każdego sprzętowego enkodera. Nie ma jeszcze pełnego HDR10/PQ/HLG ani dwupasowego HEVC w tej ścieżce.
- FFmpeg jest zewnętrzną zależnością i **nie jest spakowany z programem**.
- Obie kompilacje GitHub Actions zakończyły się sukcesem, ale wydanie DW2 pozostaje projektem rozwijanym; wcześniejsza DW1 jest stabilnym punktem odniesienia.

**Kod źródłowy użyty do kompilacji:** `1baed957bd934fb999526a7ffadc2fe14d94b6d8` (`feature/hevc-true10-dw2`).

**GitHub Actions:** [Windows x64](https://github.com/dariuszby-arch/filmcraft/actions/runs/45145672414) · [Linux AppImage i TAR.GZ](https://github.com/dariuszby-arch/filmcraft/actions/runs/45145672451)
