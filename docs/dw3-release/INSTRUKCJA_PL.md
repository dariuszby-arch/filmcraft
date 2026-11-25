# FilmCraft 0.20.2 DW3 Denoise

Wydanie testowe z przywróconą implementacją Advanced Denoise z prac DW2.
Baza: `v0.20.2-DW2`, commit `1baed957bd934fb999526a7ffadc2fe14d94b6d8`.
Wersja pakietów Rust: `0.20.2-dw3-denoise`.

## Gdzie jest odszumianie

1. Umieść film na osi czasu i zaznacz klip wideo.
2. W panelu **Efekty / Effects** rozwiń **FilmCraft DW**.
3. Kliknij dwukrotnie **Odszumianie obrazu DW / Video Denoise DW** albo przeciągnij efekt na klip.
4. W panelu **Elementy sterujące efektu / Effect Controls** rozwiń dodany filtr.
   Jeżeli jest poniżej widocznego obszaru, przewiń panel albo zwiń Ruch, Krycie i Remapowanie czasu.

Ustawienia początkowe: przestrzenne **20%**, czasowe **45%**, ochrona szczegółów **85%**, jakość **Maximum / Maksimum**.

Włącz **Przed / Po**, aby porównać tę samą klatkę. Odtwarzanie zatrzyma się,
powiększenie ustawi się na 200%. Suwak przesłony przesuwa podział; przyciski
Przed i Po pokazują wariant na całym monitorze. Zwykłe sterowanie powiększeniem
i przesuwaniem monitora pozostaje dostępne. „Przed” pomija tylko tę instancję
odszumiania na tym klipie; zachowuje inne efekty i LUT-y. Porównanie nie zmienia
projektu ani eksportu. Obraz jest pokazywany po zakończeniu obliczeń obu wariantów.

## Algorytm

Filtr CPU pracuje na liniowych wartościach RGBA float32. Zachowuje alfę,
nie obcina z założenia HDR do zakresu SDR i nie dodaje domyślnego wyostrzania.
Oddzielnie filtruje luminancję i chrominancję, chroniąc krawędzie i tekstury.
Oba suwaki siły ustawione na zero dają obraz bitowo identyczny z wejściem.

Fast i High używają maksymalnie 3 klatek, Maximum — 5. Ruch jest szacowany
na dwóch poziomach pomniejszonego obrazu, z dopasowaniem subpikselowym i kontrolą
w obu kierunkach. Niezgodne próbki są odrzucane, a po wykryciu zmiany sceny
dalsze klatki z tej strony są pomijane. Na końcach klipu jest mniej próbek.
Zamrożone klatki nie pobierają kolejnych próbek czasowych. Sąsiednie obrazy są
przetwarzane pojedynczo, aby ograniczyć wykorzystanie pamięci.

## Uruchomienie

**Windows x64:** rozpakuj ZIP do nowego katalogu i uruchom `filmcraft.exe`.
Do eksportu HEVC skopiuj swój działający `ffmpeg.exe` obok programu albo użyj
PATH / `FILMCRAFT_FFMPEG`. FFmpeg nie jest dołączony do paczki.

**MX Linux 23 x86_64:** nadaj AppImage prawo wykonywania i uruchom.
Jeżeli FUSE jest niedostępne, użyj `APPIMAGE_EXTRACT_AND_RUN=1` przed nazwą pliku.
Do eksportu potrzebny jest systemowy FFmpeg. Program zbudowano z bibliotekami
bazy Debian 12 / glibc 2.36. Nie jest to wersja uruchamiana przez Wine.

Dołączony `Przyklad_odszumiania.fcproj` zawiera własny syntetyczny wzorzec:
szarą planszę z szumem i włączonym odszumianiem. Nie wymaga zewnętrznego filmu.
W AppImage plik znajduje się w katalogu głównym po rozpakowaniu przez
`--appimage-extract`; kopia jest też w ZIP źródeł.

## Zakres wersji testowej

Odszumianie pracuje na CPU. QSV przyspiesza kodowanie, a nie ten filtr.
Maximum w 4K może wymagać renderowania podglądu; płynne odtwarzanie nie jest
gwarantowane. Procenty oznaczają siłę nastaw, nie gwarantowany procent usuniętego
szumu. Jakość na szybkim ruchu i silnym szumie trzeba ocenić na własnym materiale.
Zachowana jest ścieżka eksportu HEVC Main 10 z DW2 przez RGB48 do FFmpeg.

## Budowa ze źródeł

Wymagany Rust 1.95 i systemowe zależności opisane w bazowym repozytorium.
Na Windows: `cargo build --release --locked -p filmcraft`.
Na MX Linux: `cargo build --release --locked -p filmcraft -p filmcraft-cli`.
Do samodzielnego przygotowania AppImage służy `packaging/linux/package.sh`.
Źródła zawierają także Cargo.lock i dokumentację bazowego wydania.

