# FilmCraft 0.20.2 DW3 Denoise

Testowe wydanie forka `dariuszby-arch/filmcraft` z filtrem **Odszumianie obrazu DW / Video Denoise DW** i podglądem **Przed / Po**. Zachowuje eksport HEVC Main 10 z DW2. Udostępniane paczki to gotowe, zweryfikowane pliki z 25 listopada 2026 r.

## Co dodano

- Odszumianie przestrzenne i czasowe z ochroną krawędzi oraz szczegółów.
- Tryby Fast, High i Maximum; Maximum korzysta z maksymalnie pięciu klatek, uwzględniając ruch, zmianę sceny i granice klipu.
- Domyślne nastawy: przestrzenne **20%**, czasowe **45%**, ochrona szczegółów **85%**, jakość **Maximum**.
- Porównanie tej samej klatki przed i po filtrze: przesuwany podział albo pełny obraz Przed / Po. Porównanie zachowuje pozostałe efekty i LUT-y oraz nie zmienia eksportu.
- Praca na liniowym obrazie RGBA float32, zachowanie alfy i wartości HDR; oba suwaki siły na zero zachowują obraz wejściowy.

## Jak uruchomić odszumianie

1. Umieść film na osi czasu i zaznacz klip wideo.
2. Otwórz **Efekty / Effects → FilmCraft DW**.
3. Kliknij dwukrotnie **Odszumianie obrazu DW / Video Denoise DW** albo przeciągnij efekt na klip. Samo zaznaczenie pozycji w katalogu nie dodaje filtra.
4. W **Elementach sterujących efektu / Effect Controls** rozwiń dodany filtr. W razie potrzeby przewiń panel albo zwiń Ruch, Krycie i Remapowanie czasu.
5. Włącz **Przed / Po**. Odtwarzanie zatrzyma się, a monitor ustawi powiększenie 200%. Zaczekaj na obliczenie obu wariantów; potem użyj suwaka podziału albo przycisków Przed i Po.
6. Wyeksportuj film, aby zapisać efekt odszumiania w pliku wynikowym.

Dołączony `Przyklad_odszumiania.fcproj` jest własnym syntetycznym projektem testowym i nie wymaga zewnętrznego nagrania.

## Pliki

| Przeznaczenie | Plik |
| --- | --- |
| Windows x64 | `FilmCraft_0.20.2_DW3_Denoise_Windows_x64.zip` |
| MX Linux 23 x86_64 | `FilmCraft_0.20.2_DW3_Denoise_MX23_x86_64.AppImage` |
| Pełne źródła, patch i logi kontroli | `FilmCraft_0.20.2_DW3_Denoise_zrodla.zip` |
| Raport testów tej kompilacji | `FilmCraft_0.20.2_DW3_Denoise_raport_testow.md` |

Dodatkowo: `INSTRUKCJA_PL.md`, `Przyklad_odszumiania.fcproj`, `OPIS_WYDANIA_PL.md`, `SHA256SUMS.txt` i `RELEASE_MANIFEST.json`.

**Windows:** rozpakuj ZIP i uruchom `filmcraft.exe`. Do eksportu potrzebny jest działający `ffmpeg.exe` obok programu, w PATH albo wskazany przez `FILMCRAFT_FFMPEG`.

**MX Linux:** zainstaluj systemowy FFmpeg, nadaj AppImage prawo wykonywania i uruchom:

```bash
chmod +x FilmCraft_0.20.2_DW3_Denoise_MX23_x86_64.AppImage
./FilmCraft_0.20.2_DW3_Denoise_MX23_x86_64.AppImage
```

Bez FUSE można użyć `APPIMAGE_EXTRACT_AND_RUN=1` przed nazwą programu. Kompilacja korzysta z bazy Debian 12 / glibc 2.36.

## Wykonane testy i zakres weryfikacji

Raport z przygotowania tych paczek potwierdza **372 zaliczone testy**: 90 projektu i 282 renderera, a także formatowanie, Clippy, kompilacje Windows/Linux, podgląd Przed/Po w GUI i eksport HEVC Main 10 (`yuv420p10le`, `hvc1`) z CLI, GUI oraz AppImage. Pozostały trzy wcześniej ignorowane testy renderera. Szczegóły i ograniczenia są w załączonym raporcie oraz logach w ZIP źródeł.

Przy publikacji ponownie sprawdzono sumy SHA-256 gotowych paczek, integralność ZIP-ów oraz zgodność drzewa źródeł. Raport i pliki binarne zachowano bez zmian; zawarte w raporcie wzmianki o braku publikacji opisują stan w chwili zakończenia testów.

Odszumianie działa na **CPU**; Intel QSV przyspiesza kodowanie, a nie ten filtr. Maximum w 4K może wymagać czasu na przygotowanie podglądu. Nastawy procentowe oznaczają siłę filtra, a nie gwarantowany procent usuniętego szumu. FFmpeg nie jest dołączony.

Nowa paczka Windows została skompilowana z Linux i sprawdzona pod względem formatu oraz importowanych bibliotek; nie uruchamiano jej na natywnym Windows. AppImage sprawdzono w środowisku Linux, bez testu na fizycznym komputerze z MX Linux. Nie zweryfikowano QSV na komputerze użytkownika ani rzeczywistych nagrań z jego kamery; testowane eksporty były bez dźwięku. Filtr zachowuje wartości HDR, co nie oznacza pełnej obsługi eksportu HDR10/PQ/HLG.

## Pochodzenie źródeł

- Baza DW2: `1baed957bd934fb999526a7ffadc2fe14d94b6d8`.
- Oryginalna lokalna rewizja kompilacji: `a0d42bbc27f751a6b4825632e60e7449668afe46`.
- Commit udostępniający niezmienione źródła na GitHubie: `2f59d1d405650110a413990ec7bd5f4cb7913a68` (`release/dw3-denoise`).
- Drzewo źródeł: `88b7c8e208c5004509e7e98a69045e7d56cd9e9f`; odpowiada dokładnie zawartości ZIP źródeł, bez jego katalogu nadrzędnego.

Wersja pakietów Rust: `0.20.2-dw3-denoise`. Nie zmieniano algorytmu ani nie przebudowywano gotowych plików podczas publikacji.
