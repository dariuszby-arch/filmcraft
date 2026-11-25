# FilmCraft 0.20.2 DW3 Denoise — raport wykonanych testów

Weryfikacja zakończona: 2026-11-25T09:24:28+01:00 (Europe/Warsaw).

## Odzyskanie pracy i wersja

Po przerwaniu sesji katalog roboczy był pusty. Gotowych poprzednich plików EXE i
AppImage nie udało się odzyskać. Przywrócono zapisane wcześniej zmiany kodu
algorytmu, integrację podglądu i testy z historii wywołań narzędzi. Następnie
ponownie zbudowano paczki. Nie projektowano innego algorytmu od początku.

Baza: `v0.20.2-DW2`, `1baed957bd934fb999526a7ffadc2fe14d94b6d8`.
Lokalny commit źródeł: `a0d42bbc27f751a6b4825632e60e7449668afe46`.
Wersja pakietów: `0.20.2-dw3-denoise`. Zmieniono oznaczenia 35 pakietów
workspace; wersje zależności zewnętrznych pozostały zgodne z bazowym Cargo.lock.
W istniejącym helperze FFmpeg ograniczono mutowalność zmiennej do Windows,
usuwając ostrzeżenie kompilatora Linux bez zmiany argumentów ani działania procesu.
Po kontroli Clippy przeniesiono rozpoznanie celu podglądu do modułu Denoise,
wydzielono rysowanie marginesów monitora, aby zachować limit złożoności funkcji,
oraz uproszczono równoważny
warunek pamiętanego układu okna. Nie zmieniono algorytmu odszumiania.
Kod i paczki nie zostały opublikowane na GitHubie.

## Testy tej kompilacji

| Kontrola | Wynik |
|---|---|
| filmcraft-project, release | 90 zaliczonych, 0 błędów |
| filmcraft-render, release | 282 zaliczonych, 0 błędów |
| Odszumianie Maximum, pięć klatek 3840×2160 | Zaliczone; 2.89s, noise MSE ratio 0.162 |
| Przed/Po zachowuje inne efekty | Zaliczone w teście renderera |
| Zerowa siła, alfa, HDR, ostre krawędzie | Zaliczone |
| Przesunięcie obrazu, granice klipu, zmiana sceny, anulowanie | Zaliczone |
| Formatowanie Rust | Zaliczone |
| Clippy: projekt i renderer, wszystkie cele | Zaliczone z -D warnings |
| Clippy: biblioteka UI | Zaliczone z -D warnings |
| Kompilacja Windows x64 GNU, release | Zakończona sukcesem |
| Kompilacja natywna Linux x86_64, release | Zakończona sukcesem |
| GUI Linux: katalog efektów i Przed/Po | Zaliczone w uruchomionej aplikacji |
| Klawiatura: przycisk Przed, fokus i spacja | Zaliczone w uruchomionej aplikacji |
| Gotowy AppImage: uruchomienie, GUI i eksport | Zaliczone z APPIMAGE_EXTRACT_AND_RUN=1 |
| Eksport z CLI, GUI i AppImage | HEVC Main 10, yuv420p10le, hvc1 |
| Integralność ZIP Windows | Poprawna |
| Uruchomienie programów Linux z loaderem glibc 2.36 | Zaliczone |

Domyślne nastawy: przestrzenne 20%, czasowe 45%, ochrona szczegółów 85%, Maximum.
Przetestowano własny wzorzec: plansza szarości, efekt szumu FilmCraft i nowy filtr.
W renderowanych PNG błąd MSE względem wzorca po filtrze wyniósł
17.28% wartości sprzed filtra. Wynik dotyczy tego
syntetycznego materiału i nie stanowi gwarancji redukcji szumu w dowolnym nagraniu.

W rendererze pozostały 3 istniejące testy oznaczone wcześniej jako ignored;
nie zmieniano ich oznaczeń. Nie wykonano pełnego zestawu testów całego UI ani
WASM. Podgląd sprawdzono także przez rzeczywiste sterowanie klawiaturą w GUI.

## Zgodność i ograniczenia weryfikacji

Windows EXE ma format PE32+ x86-64 i podsystem GUI. Sprawdzono importy: tylko
biblioteki systemowe Windows, bez dodatkowych DLL runtime MinGW. Kompilowano
z Linux; nie wykonano uruchomienia tej nowej paczki na natywnym Windows.

Linux linkowano do bazy Debian 12 / glibc 2.36. Najwyższa wymagana wersja symboli
glibc: 2.35 (GUI) i
2.35 (CLI). Oba programy uruchomiono
przez loader 2.36. Nie przeprowadzono testu na fizycznym komputerze użytkownika
z MX Linux. Sesja GUI korzystała z Mesa i wirtualnego ekranu X11.

Nie testowano wszystkich kart graficznych, sprzętowego QSV na komputerze
użytkownika ani rzeczywistych nagrań z jego kamery. Odszumianie pracuje na CPU,
Maximum w 4K nie gwarantuje odtwarzania w czasie rzeczywistym. FFmpeg jest
zewnętrzny i nie jest dołączony. Testowane eksporty wideo były bez dźwięku.

## Wcześniejsza przerwana sesja

W historii narzędzi zachowały się wyniki: 90 testów projektu i 282 renderera,
test pięciu klatek 4K, kompilacje Windows/Linux oraz próby GUI, AppImage i HEVC.
Oryginalnych plików binarnych i kompletnych logów z tej sesji nie odzyskano.
Tabela powyżej podaje kontrole wykonane ponownie dla przekazywanego DW3.

## Źródła i materiał testowy

ZIP źródeł zawiera pełne drzewo projektu, Cargo.lock, zmodyfikowane pliki,
patch względem DW2, instrukcję, ten raport i logi wykonanych kontroli.
Nie zawiera katalogu kompilacji target ani danych .git. Zależności Cargo
pozostają pobierane według Cargo.lock; nie jest to paczka offline vendored.
`Przyklad_odszumiania.fcproj` jest własnym syntetycznym projektem, bez cudzych
nagrań i bez wymaganych zewnętrznych mediów.

## Sumy SHA-256 paczek

- `FilmCraft_0.20.2_DW3_Denoise_Windows_x64.zip`: `b21d9b328ec0eee0dc92da6e9c7b544c83127ad39766a50c7a958a7c357672ef`
- `FilmCraft_0.20.2_DW3_Denoise_MX23_x86_64.AppImage`: `1852b21279cfb3123eb01b1b3a03592d279297bde42266f3930b6823f6e93b68`
