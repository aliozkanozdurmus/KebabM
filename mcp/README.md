# ZaiqoM-MeetingHelper MCP

Masaüstü uygulamasını resmi MCP TypeScript SDK üzerinden yöneten yerel stdio sunucusu. Node 24 ve çalışan native uygulama gerekir. `npm run dev` tek başına yeterli değildir. MCP host'unda 72 araç, 4 kaynak, 3 kaynak şablonu ve 3 hazırlık prompt'u sunulur.

## Bağlantı

Repo kökündeki `.mcp.json`, çalışma dizini bu repo olan uyumlu host'lar içindir. Başka bir host'ta gerçek mutlak yolu kullanın:

```json
{
  "mcpServers": {
    "zaiqo": {
      "command": "node",
      "args": ["/absolute/path/to/ZaiqoM-MeetingHelper/mcp/server.mjs"]
    }
  }
}
```

Önce `npm ci` çalıştırın. Host PATH'inde Node yoksa `command` alanına Node'un mutlak yolunu yazın. Bu yapılandırma host yeniden bağlandığında etkinleşir; dosyanın bulunması tek başına bağlantı kanıtı değildir. Sunucu stdout'u yalnızca MCP protokolüne ayırır.

Tek seferlik çağrılar da gerçek MCP istemcisinden geçer:

```sh
npm run mcp:call -- status
npm run mcp:call -- --list
npm run mcp:call -- list_projects
npm run mcp:call -- search_knowledge '{"id":"PROJECT_ID","question":"Deployment pipeline nasıl çalışıyor?"}'
```

CLI hata cevabında sıfırdan farklı kodla çıkar. Kaynak içeriği özel şirket verisi içerebilir; çıktıyı kamuya açık loglara göndermeyin.

## Anahtarlar ve model seçimi

`list_configured_secrets` yalnızca kayıtlı sağlayıcı adlarını verir. `import_secret_env`, MCP sürecinin ortamında bulunan anahtarı OS credential store'a aktarır; değerini cevapta döndürmez. Anahtarı sohbete, komut satırı argümanına, repoya veya `.mcp.json` içine yazmayın. Ortam değişkenini host'un gizli değer yönetimiyle sağlayın ve yalnızca değişken adını aktarın:

```json
{"provider":"gemini","variable":"MEETINGHELPER_GEMINI_KEY"}
```

Bu JSON `import_secret_env` aracının girdisidir. Ardından `list_models` → `configure_llm` → `test_llm` kullanılır. `list_models` ve `test_llm` etkin sağlayıcı tercihini değiştirmez. Model testi gerçek istek ve kullanım oluşturur. Mevcut AI Studio anahtarını kullanmak için ilgili Google hesabına erişim gerekir; MCP giriş veya işletim sistemi izinlerini aşmaz.

macOS'ta kayıtlı anahtar listesi yalnızca Keychain metadata'sını sorgular; parolayı okumaz. Başarıyla okunan/kaydedilen anahtar, native uygulamanın ömrü boyunca bellekte tutulur; model listesi, sağlayıcı seçimi ve bağlantı testi aynı kaydı yeniden açmaz. Değiştirme/silme önbelleği günceller; bellek kaydı silindiğinde sıfırlanır. Başarısız erişimler yeniden denenebilir. Keychain dışında yapılan anahtar değişiklikleri için uygulamayı yeniden başlatın. Kilitli veya ek doğrulama gerektiren kayıtlar durum listesinde görünmeyebilir; gerçek erişim ayrıca sınanır.

Yerel ad-hoc derleme kalıcı geliştirici kimliği değildir: uygulama yeniden derlendiğinde ilk anahtar erişiminde macOS yeniden izin isteyebilir. Güncellemeler arasında kalıcı güven için aynı Apple Developer ID ile imzalanmış dağıtım gerekir. Bu davranış Keychain ACL'lerini gevşeterek veya anahtarı düz metin dosyaya taşıyarak çözülmez.

Cevap üretimi, embedding, STT ve çeviri ayrı yapılandırılır. Gemini cevap anahtarının varlığı Deepgram gibi başka bir STT sağlayıcısını yapılandırmaz. `set_reply_language`, STT dilini değiştirmez. `configure_translation` yalnızca çeviri tercihlerini günceller.

## Günlük akışlar

1. **Hazırlık:** `status`, `list_projects`, `knowledge_status`. Gerekiyorsa `scan_project`; ardından `search_knowledge` ve `read_evidence`. Kaynakları dosya/satır/revizyonla aktarın. Repo içeriğini canlı production veya müşteri teslimi kanıtı saymayın.
2. **Semantik arama:** `get_embedding_config`, `set_embedding_config`, `embed_project`. Bulut embedding seçilirse seçilen projenin kaynak parçaları o sağlayıcıya gönderilir. Sözcük araması embedding olmadan da çalışır.
3. **Ses kontrolü:** `list_audio_devices`, `start_audio_test`, kısa konuşma/video oynatma, mutlaka `stop_audio_test`. Başlatma cevabı ses algılandığını kanıtlamaz; durdurma sonucu `audioDetected` bunu bildirir. Test kendiliğinden kapanmaz. macOS ekran/sistem sesi izni gerekir.
4. **Toplantı:** `start_meeting`, uygun STT ile `start_capture`, `get_session`, `ask_question` veya `answer_question`, gerektiğinde `cancel_assist`, son olarak `end_meeting`. Toplantı açmak ses yakalamayı başlatmaz. Yinelenen aktif toplantı reddedilir. `web_speech` tarayıcıya bağlıdır ve MCP üzerinden capture için desteklenmez.
5. **Takip:** `get_meeting`, `draft_meeting_decisions`, `project_memory`. Karar taslaklarını yalnızca kullanıcı incelemesinden sonra `review_decision` ile onaylayın. `save_open_question` ve `recheck_questions` eksik yanıtları korur; yeni kanıt soruyu otomatik çözmez.

`start_capture` içinde `you.role` tam olarak `You`, `them.role` tam olarak `Them` olmalıdır. Kaynak başına cihaz ve STT sağlayıcısı belirlenir; `language` bağımsız STT dilidir. Kaydın diske yazılması mevcut `recordingEnabled` tercihine bağlıdır.

## Uzun işler ve iptal

Kalıcı MCP bağlantısında `start_job` şu işlemleri başlatır: `scan_project`, `embed_project`, `project_preparation`, `draft_meeting_decisions`, `ask_question`, `answer_question`. Önce girdiler doğrulanır. Dönen ID ile `get_job` çağrılır; durum `running`, `completed`, `error` veya `cancelled` olur. `cancel_job` aktif isteği iptal eder; tamamlanmış yazıları geri almaz.

Aynı anda en fazla 4 iş ve bağlantı başına en fazla 40 iş kaydı tutulur. Bağlantı kapanırsa iş kayıtları kaybolur ve devam eden istekler iptal edilir. Yerel bloklayan bir işlem tamamlanmış olabilir; tekrar denemeden native durumu okuyun. Tek seferlik CLI iş araçlarını reddeder; orada doğrudan asıl işlemi çağırın. Normal çağrı süresi 3 dakika, uzun işlem sınırı 30 dakikadır. Yerel model indirmeleri uygulamaya aittir; durum için `list_local_stt_models`, iptal için `cancel_model_download` kullanılır.

## Kaynaklar ve prompt'lar

- `meetinghelper://status`, `meetinghelper://projects`, `meetinghelper://settings`, `meetinghelper://session`
- `meetinghelper://projects/{id}`, `meetinghelper://meetings/{id}`, `meetinghelper://evidence/{id}`
- `prepare_meeting`, `return_to_work`, `diagnose_readiness`: isteğe bağlı `project_id` ve `language` alır. Prompt sunulması kendi başına test veya AI üretimi çalıştırmaz.

## Erişim ve test sınırları

Sunucu yalnızca `127.0.0.1:47331–47333` üzerindeki uygulama kontrol API'sine bağlanır. Kontrol token'ı uygulamanın özel dosyasından okunur; yanıtlarda gösterilmez. macOS/Linux'ta dosyanın grup/diğer kullanıcı erişimi reddedilir. API Origin taşıyan ve geçersiz token'lı istekleri reddeder. Windows kullanıcı ACL kabulü henüz açık koşuldur. Bu yerel erişim modeli güvenilmeyen MCP host'ları için bir yetki ayrımı sağlamaz; bağlı host proje verisini okuyabilir ve araçlarla değiştirebilir.

```sh
npm run test:mcp
npm run check
```

Testler resmi SDK ile eski/güncel bağlantı, şema reddi, kaynak/prompt keşfi, hata maskeleme, gizli değer aktarımı, uzun iş sonucu/iptali ve gerçek HTTP bağlantısının iptalini kapsar. Bunlar sağlayıcı kalitesi veya gerçek Zoom ses testi değildir. 2026-09-27 yerel native denemede toplantı açma/kapatma, yinelenen oturum reddi, yapılandırılmamış AI hatasından sonra oturumun korunması doğrulandı. YouTube sistem sesi denemesi macOS TCC izninde durdu; gerçek AI testi Zaigo Google hesabı yeniden girişini bekliyor.

Tam kabul durumu: [plan.md](../plan.md), [kabul protokolü](../docs/acceptance.md).

### Appearance control

`set_settings` accepts `{"values":{"appearance":"liquid-glass","theme":"dark"}}`.
Supported appearances: `ibm`, `liquid-glass`, `apple`, `linear`, `notion`, `material`,
`github`, `terminal`. Color mode (`light`, `dark`, `system`) is independent. Read current
settings before updating; the selection is persisted and synchronized across windows.
