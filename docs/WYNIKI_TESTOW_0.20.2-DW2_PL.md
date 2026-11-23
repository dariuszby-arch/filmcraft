# FilmCraft 0.20.2-DW2 — wyniki naszych testów obrazu

**Zakres:** porównania HEVC Main (8-bit SDR) i HEVC Main 10 (10-bit SDR), przekazanie użytkownika: 23 listopada 2026. Dane pochodzą z `ffprobe`, `MediaInfo` oraz trzech par klatek PNG. Porównanie klatek wykonano dla przesłanych obrazów PNG, a nie bezpośrednio dla całych filmów.

## Test 1 — Full HD

Ten sam materiał, 1920 × 1080, 26,794 s, kodek HEVC/x265, wybór `Auto (Intel QSV / CPU)`, zadany `CBR 20 Mb/s`.

| Właściwość | `8bit.mp4` | `10bit.mp4` |
| --- | ---: | ---: |
| `codec_name` | hevc | hevc |
| `profile` | Main | Main 10 |
| `pix_fmt` | yuv420p | yuv420p10le |
| `stream.bit_rate` | 12 599 815 bit/s | 4 148 756 bit/s |
| `format.bit_rate` | 12 918 635 bit/s | 4 473 021 bit/s |
| `duration` | 26,794 s | 26,794 s |
| `format.size` | 43 267 739 B | 14 981 268 B |

Wynik: rozmiar 8-bit / 10-bit = ok. **2,89×**, średni bitrate wideo = ok. **3,04×**. 10-bitowy plik był o **65,4% mniejszy**. Nie przypisujemy całej różnicy wyłącznie głębi 10-bit: rzeczywiste bitrate są różne mimo tych samych ustawień wejściowych.

MediaInfo wykazał dla **obu** plików: `x265 - 3.5+1-f0c1022b6`, `rc=cbr`, `bitrate=20000`, `vbv-maxrate=20000`, `vbv-bufsize=40000`, `no-strict-cbr`. Wybór „Auto” w tych próbach zakończył się kodowaniem programowym x265. Oba profile to odpowiednio `Main@L4@High` oraz `Main 10@L4@High`.

Porównane PNG `10bit.png` (RGBA 1920×1080) i `8bit.png` (RGB 1920×1080), po sprowadzeniu do RGB:

- SSIM klatka do klatki: **0,983564**.
- PSNR klatka do klatki: **28,993 dB**.
- Średnia różnica kanałów 10-bit minus 8-bit: **R +7,74; G +6,52; B +10,05** (skala 0–255).
- Zmiana średniej jasności jest widoczna w liczbach; może wynikać z konwersji albo ustawień. Bez źródłowej klatki nie można powiedzieć, który eksport wierniej odtwarza oryginał.
- Użytkownik nie widział istotnej różnicy podczas zwykłego oglądania.

## Test 2 — UHD 4K

Zadane ustawienia wyjściowe: 3840 × 2160, 23,976 fps, 70,40 s, H.265 HEVC, CBR 20 Mb/s, Auto (Intel QSV / CPU), AAC 320 kb/s / 48 kHz stereo, keyframe co 48 klatek. Testowano 8-bit SDR i 10-bit SDR z tym samym materiałem. Użytkownik stwierdził, że **rozmiary plików są niemal identyczne**; brak osobnych liczbowych wyników `ffprobe` dla tej próby.

Zrzuty: `10bit_org.png` (oznaczony jako oryginalny), `10bit(1).png` (HEVC 10-bit), `8bit(1).png` (HEVC 8-bit). Obrazy mają 2048 × 1152 pikseli i format RGBA.

| Pomiar dla klatki względem `10bit_org.png` | Eksport 10-bit | Eksport 8-bit |
| --- | ---: | ---: |
| SSIM | 0,959747 | 0,965423 |
| PSNR | 38,565 dB | 39,511 dB |
| Średnia różnica R (0–255) | +0,531 | −0,522 |
| Średnia różnica G (0–255) | +0,572 | −0,792 |
| Średnia różnica B (0–255) | +0,577 | −0,554 |

W tej jednej klatce 8-bit wypada nieco bliżej zrzutowi oznaczonemu jako oryginalny. To **nie** dowodzi wyższej jakości 8-bitowego całego filmu: screenshoty są mniejsze niż 4K i po eksporcie do 8-bitowego PNG informacja o 10-bitowych gradientach została ograniczona. Konieczne jest porównanie źródła i obu filmów bez dodatkowej konwersji, przy jednakowym zarządzaniu kolorami.

## Ograniczenia pomiarów

- Podane SSIM i PSNR liczyliśmy na dostarczonych **statycznych PNG** z użyciem RGB i standardowych funkcji obrazu (`skimage.metrics`); nie są to wyniki VMAF ani statystyki całego filmu.
- Test 1 nie ma klatki referencyjnej, dlatego SSIM=0,983564 opisuje jedynie podobieństwo dwóch wyników.
- Test 2 ma tylko jedną referencyjną klatkę PNG o rozdzielczości 2048×1152 — nie jest to analiza oryginalnego strumienia 4K.
- Dla x265 `rc=cbr` w metadanych **nie oznacza** ścisłej średniej 20 Mb/s; oba wyniki potwierdziły `no-strict-cbr`.
- Różne wartości bitrate mogą silniej wpływać na jakość i wielkość plików niż sama głębia 8/10-bit.

## Proponowane dalsze testy

1. Oryginalny film + oba eksporty o identycznym fps i klatkach: porównanie pełnego VMAF / SSIM / PSNR, z kontrolą transformacji kolorów.
2. Klatki 10-bitowe wyciągane do odpowiednio wysokoprecyzyjnego formatu bez wymuszania PNG 8-bit oraz gradient testowy przed/po kodowaniu.
3. Osobne sprawdzenie ścisłego CBR x265 i Intel QSV oraz pracy na dłuższych, dynamicznych fragmentach 4K.

**Stan:** udane kompilacje GitHub Actions DW2 na Windows i Linux; wyniki powyżej uzyskano w testach użytkownika. Nie przedstawia się ich jako certyfikatu zgodności czy dowodu przewagi 10-bitowego kodowania.
