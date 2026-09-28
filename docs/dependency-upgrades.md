# Bağımlılık güncellemeleri ve istisnalar

Kontrol tarihi: 2026-09-27. npm/crates.io resmi katalogları, manifestler ve kilitler karşılaştırıldı. Rust envanteri, aynı adlı transitif paketlerden değil, `cargo metadata` içindeki kök paketin gerçek bağımlılık kenarlarından çıkarıldı: **51 doğrudan bağımlılık; 50'si kontrol günündeki kararlı katalog sürümünde, ORT açık RC istisnası.** Bu sayı bütün transitif paketlerin en yeni sürümde olduğunu veya iki platformdaki ürün kabulünün geçtiğini göstermez.

## Frontend ve geliştirme araçları

Node 24 ve `@types/node` 24 hizalandı. `npm ci` kilitli kurulum, CI ise Node 24 ve kilitli Cargo testleri kullanır. Kurulu başlıca sürümler: React 19.3.0, TypeScript 7.0.2, Vite 8.3.1, Tauri JS API/CLI 2.12.0, Playwright 1.63.0 ve Vitest 5.0.2.

`@types/node` 26, seçilen Node 24 hattıyla uyum için alınmadı. Tauri 3 alpha sürümleri seçilmedi. npm audit: 0 bildirilen açık. RustSec taraması yapılmadı; npm sonucu native bağımlılık güvenliğine genellenemez.

## Tamamlanan Rust geçişleri

| Grup | Doğrudan kilitli sürümler | Yapılan uyarlama ve kontrol |
|---|---|---|
| Tauri | tauri 2.12.0, tauri-build 2.7.0 | JS/Rust eşleştirmesi; macOS native build ve testler. |
| Hash/arşiv | sha2 0.11.0, zip 8.6.0, bzip2 0.6.1, base64 0.23.1 | SHA-256 metin biçimi korunur; DOCX, model arşivi ve bozuk dosya fixture'ları. |
| Veri/doküman | rusqlite 0.40.2, pdf-extract 0.12.1, lopdf 0.45.0 | SQLite sayısal dönüşümleri, FTS ve migration/yedek testleri; PDF kaynak API uyarlaması ve metin fixture'ı. |
| Ağ | reqwest 0.13.5, tokio-tungstenite 0.30.0, thiserror 2.0.21 | Query/form özellikleri açık; SSE/UTF-8, hata ve iptal testleri. Gerçek STT yeniden bağlantı kabulü açık. |
| Kimlik | keyring 4.2.0 | Native macOS saklama backend'i, eski hizmet adları; sentetik Keychain kalıcılık testi. |
| Ses | cpal 0.18.2, screencapturekit 11.0.0, libopus_sys 0.4.0 | Yeni aygıt/config API'leri, ortak PCM dönüşümü, kontrollü ses tamponu ve Opus round-trip. Gerçek aygıt/Zoom kabulü açık. |
| Windows API | windows 0.62.2 | PROPERTYKEY konumu, credential flag'leri, WinRT async/event API'leri; seçili Windows modüllerinin MSVC hedefinde derleme kontrolü. |
| Yerel çeviri/ML | tokenizers 0.23.2, ndarray 0.17.2 | Null normalizer içeren tokenizer fixture'ı, Unicode ve özel token kimlikleri; gerçek Marian/ONNX model turu açık. |

Kullanılmayan doğrudan `wasapi` bağımlılığı kaldırıldı; Windows loopback için CPAL'in WASAPI backend'i kullanılıyor. `pdf-extract` kendi uyumlu transitif `lopdf` sürümünü tutabilir; buna doğrudan `lopdf` yükseltildi diye zorla override uygulanmadı.

Eski keyring 3 yapılandırmasında native store özelliği seçili değildi. keyring 4'ün native backend'iyle kalıcı saklama etkin. `zaiqoM`, önceki `Zaiqo` ve `NexQ` hizmet kimlikleri korunur. Sentetik test eski hizmet adına test kaydı oluşturur, uygulama adaptöründen okur, yeni kimliğe yazar, ayrı `security` sürecinden varlığını doğrular ve temizler. Bu test kullanıcının mevcut kayıtlarının ACL izinlerini değiştirmez veya bunların yeni binary'ye erişim onayı verildiğini kanıtlamaz. Bellekte tutulmuş ve kaybolmuş eski anahtarlar geri getirilemez.

## Açık istisnalar ve kabul sınırları

- `ort` **tam olarak `=2.0.0-rc.13`** sürümüne sabitlendi. Kullanılan 2.x API için kararlı katalog sürümü yok; 1.x'e sessizce geri dönülmedi. `ndarray` 0.17 hattı ORT ile uyumlu tutuldu.
- Yerel macOS testleri: 74 unit + 4 uyumluluk testi geçti. İsteğe bağlı, varsayılan atlanan 1 native Keychain testi ayrıca geçti. Derleme uyarısı yok; frontend paketleyicinin statik/dinamik import uyarıları ayrı.
- Windows'ta gerçek uygulama linkleme, installer, Credential Manager, aygıt ve Zoom testi henüz yapılmadı. macOS'tan Windows hedefinde yapılan kontrol yalnızca seçili ses/STT/kimlik modüllerini kapsıyor.
- Windows Speech adaptörü yalnızca desteklediği mikrofon yolunu kullanır. Desteklenmeyen PCM/sistem sesi yolunda sahte konuşma metni üretmez; açık hata verir. Bu taraf için Deepgram veya PCM kabul eden yerel sağlayıcı seçilmelidir.
- Gerçek Whisper/Marian model dosyalarıyla uçtan uca çıkarım, iki platformda 60 dakika toplantı ve imzalı updater/geri dönüş hâlâ kabul koşullarıdır.

## Kanıt ve yenileme

Yerel envanterler `artifacts/dependencies-rust.json`, `artifacts/dependencies-npm-outdated.json`, `artifacts/dependencies-npm-audit.json` altında, Git dışında tutulur. Yeni uygulama gününde kataloglar yeniden kontrol edilmeli. Kod kontrolleriyle gerçek platform kabulü [plan.md](../plan.md) ve [kabul protokolünde](acceptance.md) ayrı takip edilir.

- [npm outdated](https://docs.npmjs.com/cli/v11/commands/npm-outdated)
- [Cargo metadata](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html)
- [CPAL 0.18 geçişi](https://docs.rs/crate/cpal/0.18.2/source/UPGRADING.md)
- [ScreenCaptureKit 11](https://docs.rs/screencapturekit/11.0.0/)
- [keyring 4.2](https://docs.rs/keyring/4.2.0/)
- [Tauri dağıtım](https://v2.tauri.app/distribute/)
