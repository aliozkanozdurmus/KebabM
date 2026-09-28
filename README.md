# ZaiqoM-MeetingHelper

Seçilen projenin kodunu, dokümanlarını ve kayıtlı toplantılarını kullanarak toplantı sırasında kısa, kaynaklı cevaplar hazırlayan kişisel masaüstü asistanı. Tauri 2 + Rust + React. macOS, Windows ve Linux masaüstü paketleri.

**Durum:** Yerel geliştirme sürümü. macOS ARM64 uygulaması ve DMG derlendi; gerçek Zoom/sağlayıcı ve Windows kabulü henüz tamamlanmadı. Uygulanan işler, ölçümler ve açık kalan koşullar [plan.md](plan.md) içinde takip ediliyor.

## Başlatma

Node 24, Rust stable, platform geliştirme araçları, CMake ve Ninja gerekir. macOS'ta Xcode Command Line Tools; Windows'ta Visual Studio C++ Build Tools, WebView2 ve LLVM/libclang gerekir. `.nvmrc` ve kilit dosyaları sürümleri sabitler.

```sh
npm ci
npm run tauri -- dev
```

`npm run dev` yalnızca frontend sunucusudur; gerçek ses ve Tauri komutları için native uygulamayı kullanın.

## MCP ile yönetim

Toplantı, kaynak araması, sağlayıcılar, ses, çeviri ve hazırlık işlerini 72 MCP aracıyla yönetebilirsiniz. Bağlantı, güvenli anahtar aktarımı, uzun işler ve örnek akışlar: [MCP kılavuzu](mcp/README.md). `npm run mcp:call -- status` çalışan native uygulamayı gerçek MCP istemcisiyle sınar.

## İlk toplantı

1. **Projects & preparation** ekranında çalışacağınız repo klasörünü seçin ve **Update index** çalıştırın. `.github` iş akışları da taranır. Kapsamı, dışlanan dosyaları ve güncellik durumunu gözden geçirin.
2. **Settings** bölümündeki AI grubunda sağlayıcı/anahtar/modelinizi seçin. Mevcut ayarlar korunur. Yeni profil önerisi otomatik bağlantı veya kota sağlandığı anlamına gelmez.
3. İsterseniz projenin **Search provider** bölümünde Gemini veya Ollama embedding seçip **Save & build semantic index** çalıştırın. Gemini seçildiğinde o projenin kaynak parçaları API'ye gönderilir. Yapılandırılmamış kurulum sözcük aramasıyla çalışır.
4. **Start meeting → Check before joining** ile mikrofonu, karşı taraf sesini, seçilen modeli, STT bağlantısını ve proje indeksini ayrı ayrı sınayın. STT bağlantı kontrolünden sonra gerçek konuşma denemesi yapın.
5. Canlı ekranda soruyu yazın veya otomatik yardımı kullanın. Yeni cevap, okunan cevabı değiştirmez. Kaynaklar indekslendiği andaki içerik ve revizyonla açılır.

STT dili, cevap dili, çeviri dilleri ve sağlayıcılar bağımsızdır. Kaynakta kod bulunması, özelliğin production'da açık olduğunu kanıtlamaz.

## Kontroller ve paketleme

```sh
npm run check
npm run test:e2e
cargo test --locked --manifest-path src-tauri/Cargo.toml
npm run app
```

macOS'ta `cargo test` için gerekirse `LIBCLANG_PATH=/Library/Developer/CommandLineTools/usr/lib` ve Homebrew/Cargo PATH eklenmelidir. `npm run tauri` bunları macOS'ta otomatik bulur. Finder otomasyonu olmayan ortamda `CI=true npm run app` DMG'nin dekorasyon adımını atlar. Çıktılar `src-tauri/target/release/bundle/` altındadır; kaynak ağacında EXE tutulmaz.

Seçilen projenin sabit revizyonunda, repo içeriğini değiştirmeyen yerel arama değerlendirmesi:

```sh
cargo run --locked --manifest-path src-tauri/Cargo.toml --example evaluate_knowledge -- /path/to/repo artifacts/evaluation.json /path/to/private-golden.json
```

Bu komut 40 cevaplanabilir + 10 sınırlılık senaryosunu kullanır. Arama eşiği karşılanmazsa hata koduyla çıkar; AI cevap kalitesini ölçmüş sayılmaz. `artifacts/` dizinini ilk çalıştırmadan önce oluşturun.

## Veri ve sürüm yönetimi

Eski `com.nexq.app`, `nexq.db` ve keychain kimlikleri korunur. Yeni indeksler uygulama verisinde tutulur; kaynak repoya otomatik dosya yazılmaz. SQLite v12 geçişinden önce mevcut veritabanının yedeği alınır. Başarısız tarama önceki indeksi bırakır.

Mac/Windows kontrol ve paket iş akışları `.github/workflows/` altındadır. `npm run release:dry-run` sürüm hazırlığını gösterir; `npm run release` yalnızca yerel sürüm/changelog dosyalarını hazırlar, commit/tag/push yapmaz. İmzalı updater, anahtar ve HTTPS endpoint yapılandırılana kadar kapalıdır. [Dağıtım, yedek ve geri dönüş adımları](docs/acceptance.md), [bağımlılık istisnaları](docs/dependency-upgrades.md).


## Download desktop installers

Published installers and SHA-256 checksums are available on [GitHub Releases](https://github.com/aliozkanozdurmus/ZaiqoM-MeetingHelper/releases). Windows uses an NSIS `.exe`, macOS Apple Silicon uses a `.dmg`, and Linux x64 uses `.AppImage` or `.deb`. Initial packages do not have trusted publisher signing/notarization; signed automatic updates remain disabled. Build status and remaining runtime acceptance are documented in each release.
