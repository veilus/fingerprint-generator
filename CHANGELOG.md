# Changelog

## 0.3.1 — 2026-09-19

**Sửa một khiếm khuyết do chính 0.3.0 phát hành ra.** Không đổi chữ ký công khai nào.

### Hồ sơ Windows khai tới 640 nhân — con số Chrome thật không phát ra được

0.3.0 nới `hardware_concurrency` lên `u16` và cắt theo hệ điều hành, nhưng
**không đặt trần cho Windows**, với lý do *"server thật đạt tới hàng trăm nhân"*.
Lý do đó đúng về **phần cứng** và sai về **Chrome**.

`navigator.hardwareConcurrency` trả thẳng `base::SysInfo::NumberOfProcessors()`:

```
navigator_concurrent_hardware.cc:11
  return static_cast<unsigned>(base::SysInfo::NumberOfProcessors());
```

Trên Windows hàm đó lấy `dwNumberOfProcessors` từ `::GetNativeSystemInfo`:

```
sys_info_win.cc          NumberOfProcessors() -> OSInfo::processors()
windows_version.cc:174   processors_ = system_info.dwNumberOfProcessors
windows_version.cc:71    ::GetNativeSystemInfo(&info)
```

`dwNumberOfProcessors` chỉ đếm bộ xử lý logic **trong một processor group**, và
Windows giới hạn mỗi group 64. Chrome **không** gọi
`GetActiveProcessorCount(ALL_PROCESSOR_GROUPS)`. Nên một Chrome thật trên Windows
không bao giờ báo quá **64**, dù máy có 640 luồng.

Trần mới `WINDOWS_MAX_CORES = 64` — **không** phải giới hạn phần cứng, mà là giới
hạn của chính Chrome, đọc từ cây nguồn Chromium 153.0.8010.37.

Linux giữ nguyên không trần: `sysconf(_SC_NPROCESSORS_ONLN)` trả số thật, không
có khái niệm processor group.

### Cách phát hiện, vì nó đáng hơn bản sửa

`tools/fingerprint-parity` — bộ đo đối chiếu thư viện với các phép kiểm nhất quán
của Veilus — chạy **sau** khi 0.3.0 đã lên crates.io. Đo trước/sau, cùng 1500 hồ sơ:

```
                                   truoc 0.3.0    0.3.0    0.3.1
Hardware concurrency plausible          0.0%       1.1%     0.0%
Cores and RAM plausible together       25.3%      28.1%    28.1%
```

Vế thứ nhất là khiếm khuyết này, nay đã hết. Vế thứ hai chưa hết — xem dưới.

### Còn lại, KHÔNG sửa ở bản này

`Cores and RAM plausible together` vẫn 28,1% so với 25,3% trước 0.3.0. Tỉ lệ lệch
trải đều mọi hệ điều hành, đo trên 1500 hồ sơ:

```
Linux 43.8%   Windows 28.4%   MacOs 28.3%   Ios 24.0%   Android 18.7%
```

Đây là tính chất **cấu trúc** của mạng Bayes: nó lấy mẫu `hardwareConcurrency` và
`deviceMemory` mà không ghép cặp, nên hồ sơ nhiều nhân thường nhận ít RAM. 0.3.0
chỉ làm biên độ số nhân lớn hơn nên tỉ lệ lộ rõ hơn 2,8 điểm. Sửa nó là đổi cách
lấy mẫu, không phải đổi một trần — việc riêng.

## 0.3.0 — 2026-09-19

**HAI THAY ĐỔI PHÁ VỠ**, gộp một bản có chủ đích để người dùng ngoài chịu đúng
một lần vỡ thay vì hai:

```
NavigatorFingerprint::max_touch_points       Option<u8>  ->  Option<u16>
NavigatorFingerprint::hardware_concurrency           u8  ->          u16
```

Người dùng khớp kiểu tường minh (`let x: u8 = fp.navigator.hardware_concurrency;`)
phải sửa một dòng mỗi chỗ; người dùng chỉ đọc giá trị thì không phải làm gì.

Cả hai cùng một gốc: **dữ liệu của chính thư viện không lọt qua kiểu của chính
nó**, và mất im lặng. Cùng lúc, `hardware_concurrency` nay bị cắt theo hệ điều
hành — xem mục riêng bên dưới, vì đó là thay đổi HÀNH VI chứ không chỉ kiểu.

### Vì sao đổi: dữ liệu của chính thư viện không lọt qua kiểu của chính nó

Mạng Bayes kèm theo crate khai `256` là một giá trị có thể xảy ra của nút
`maxTouchPoints`:

```
possibleValues = 0, 1, 2, 3, 5, 9, 10, 20, 40, 256
```

`256` không lọt vào `u8`. Bộ đọc trả `None`, và `None` ở đây **không phân biệt
được với "mạng không có dữ liệu"**. Người dùng đọc ra một câu sai — rằng thư
viện không biết gì về hồ sơ này — trong khi thư viện biết, và con số đó là số đo
từ traffic thật.

Nó không nổ, không cảnh báo, không trả `Err`. Đó là lý do nó sống lâu.

### Bài kiểm mới, và điều kiện để nó có nghĩa

`moi_gia_tri_maxtouchpoints_trong_mang_deu_doc_lai_duoc` khẳng định **mọi** giá
trị trong `possibleValues` của nút đó đều đọc lại được. Bài này đã được chạy ở
trạng thái ĐỎ trước khi sửa:

```
mang khai 10 gia tri cho maxTouchPoints nhung 1 gia tri khong doc lai duoc:
["*STRINGIFIED*256"]
```

Nếu viết sau khi sửa thì nó chỉ đang nói `u16` chứa được số nhỏ — một điều không
ai nghi ngờ. Bài kèm một hàng đối chứng: nút có dưới 5 giá trị thì bài tự đỏ,
để việc thu gọn dữ liệu không âm thầm làm nó vô nghĩa.

### `hardwareConcurrency` — cùng lớp lỗi, nặng hơn 150 lần (VEIL-738)

`NavigatorFingerprint::hardware_concurrency` đổi từ `u8` sang `u16`. Bốn giá trị
của mạng vượt `u8` (384, 448, 512, 640) chiếm **26,36%** khối lượng xác suất
toàn mạng, và chúng rơi vào `.unwrap_or(4)` — tức mất thành một con số **trông
hợp lý**, không phân biệt được với một máy 4 nhân thật.

Nhưng nới kiểu một mình là **sai**, và phép đo nói rõ vì sao. Với mỗi giá trị
vượt `u8`, hệ điều hành của UA đi kèm:

```
384   Linux=72.00   macOS=46.24   Windows=8.00
448/512/640         ~0.04 tong cong — gan nhu khong ton tai
```

384 luồng trên server Linux là **có thật** (EPYC 9754 hai socket = 256 nhân /
512 luồng). 384 nhân trên một máy Mac thì **không tồn tại**. Nên phát nguyên giá
trị sẽ tạo một dấu vân tay MỚI, tệ hơn dấu vân tay sai nó đang thay.

Cách chọn: nới kiểu **và** cắt theo hệ điều hành mà hồ sơ khai.

```
macOS     24   Mac Pro M2 Ultra
iOS       10   iPad Pro M4
Android   16   TRAN CO BIEN, khong phai so nhan cua mot may cu the
Windows        khong cat — server that dat toi hang tram nhan
Linux          khong cat — cung ly do
```

**Điều này cũng đổi hành vi có sẵn, không chỉ khôi phục dữ liệu mất.** Trước bản
này, hồ sơ iOS và Android đã nhận tới 186–192 nhân, vì `u8` cho lọt mọi giá trị
25–255. Nới kiểu chỉ **phơi ra** lỗi khả dĩ đó chứ không tạo ra nó. Đo trên 3000
hồ sơ, trước và sau:

```
           truoc              sau
Ios        max 640, 20.4% >24    max 10
Android    max 512, 19.0% >24    max 16
MacOs      max 384                max 24
Linux      max 144                max 144   khong doi
Windows    max 640                max 640   khong doi
```

**Góc cắt có ý, kèm trần:** mọi hồ sơ macOS vượt 24 đều về **đúng** 24, nên giá
trị 24 bị dồn cao hơn tự nhiên. Đó vẫn là một dấu vết — chỉ nhỏ hơn dấu vết cũ
(mọi thứ về 4). Nâng cấp khi có lý do thật: rút lại từ phân phối có điều kiện
của chính nút đó thay vì cắt cứng.

**Ba bài mới, hai trong số đó chạy ở trạng thái ĐỎ trước khi sửa.** Bài thứ ba
là bài **đầu-cuối** — sinh hồ sơ thật rồi kiểm trần — và nó tồn tại vì một lý do
đo được: bản sửa đầu tiên chỉ lọc macOS, **mọi bài đơn vị đều xanh**, và chính
phép đo đầu-cuối mới lộ ra iOS còn nhận 640 nhân. Đột biến xác nhận khoảng mù
đó: gỡ lời gọi lọc khỏi đường sinh mà giữ nguyên hàm → bài đơn vị **vẫn xanh**,
chỉ bài đầu-cuối đỏ.

### Thứ bản này KHÔNG sửa

`tools/fingerprint-parity/README.md` ghi phép kiểm *"Cores and RAM plausible
together"* trượt **25,3%**, quy nguyên nhân cho lệch niên đại dữ liệu. Con số đó
nằm sát **26,36%** đo được ở đây. Chưa kiểm hai cái có phải một; nếu đúng thì
chẩn đoán cũ đang chỉ sai chỗ.

## 0.2.4 — 2026-09-17

Sửa lỗi. **Không đổi chữ ký công khai nào** — chỉ siết một bộ lọc nội bộ.

### Hồ sơ Windows mang renderer của OS khác — 16% số lượt

Đo qua harness parity của Veilus, 500 seed × 3 OS chạy qua 22 phép kiểm nhất
quán:

```
ho so Windows mang renderer OS khac   81/500 (16%)  ->  0/500
```

Renderer lọt vào là chuỗi **macOS** (`Intel Iris OpenGL Engine`, cách đặt tên
driver của Apple), chuỗi **Mesa/Linux**, và renderer **trần** không có vỏ
`ANGLE (...)`.

### Nguyên nhân: một lệ đúng đặt trên một giả định sai

`renderer_mau_thuan_os` chỉ loại Apple Silicon cho Windows, theo lệ *"chỉ loại
thứ chắc chắn sai"*. Lệ đó đúng. Giả định bên dưới thì sai: rằng renderer
Intel/NVIDIA/AMD **trần** có thể là Windows.

Hai nguồn độc lập đều nói không:

```
bang GPU curated cua Veilus   21/21 muc Windows bat dau "ANGLE ("
tap Apify                     383 renderer ANGLE+Direct3D
```

Chrome trên Windows đi qua ANGLE từ đầu. Trên **Linux** thì renderer trần *là*
hợp lệ — 5/11 mục bảng Linux không ANGLE — nên **chỉ Windows** đòi khớp dương.

Đa dạng không mất: 395/650 giá trị còn dùng được cho Windows.

### Một mối nguy còn lại, ghi ra thay vì vá mù

11/80 UA Windows+Chrome có CPT `videoCard` **không chứa renderer Windows nào**.
Ở những UA đó bộ lọc giao rỗng, và `traverse_cpt_filtered` rơi về tập **chưa
lọc** — bốc đúng thứ vừa loại bỏ, không một dòng log nào.

Hôm nay không với tới được: phép kẹp UA không chọn những UA ấy, và một bài quét
500 seed cho `0/500` kể cả khi tắt mọi cơ chế vớt lại. Nên **không** thêm mã
vớt — một đoạn sửa không phép kiểm nào phân biệt được là mã đầu cơ. Khi nó với
tới được, thứ cần sửa là **sự im lặng** ở `sampler.rs`, không phải thêm một lớp
vá ở bộ lọc.

## 0.2.3 — 2026-09-16

Sửa lỗi. **Không đổi chữ ký công khai nào** — hai bản vá dưới đây đi qua
`deserialize_with`, nên kiểu vẫn y nguyên.

### Hai trường luôn trả `None`, và không ai biết

`assembler::parse_stringified` kết thúc bằng `.ok()`. Một kiểu không khớp hình
dạng dữ liệu vì thế trả `None` và **không báo gì**. Đo trên 1500 hồ sơ
(500 seed × 3 OS):

```
video_card · audio_codecs · video_codecs · fonts · mock_web_rtc  100%
battery                                                          99,1%
plugins_data                                                      7,3%   <- hỏng
multimedia_devices                                                  0%   <- hỏng
```

Cả hai được README đánh dấu ✅ trong bảng feature-parity suốt thời gian đó.

**`multimedia_devices` — 0/1500.** Kiểu khai `speakers/micros/webcams: u8`,
nhưng dữ liệu Apify mang **mảng đối tượng thiết bị**:
`{"speakers":[{...}],"micros":[...],"webcams":[...]}`. Nay đếm độ dài mảng,
đúng như README vẫn mô tả (*"Number of audio output devices"*). Bộ giải mã nhận
**cả mảng lẫn số**, để nếu thượng nguồn đổi sang ghi số đếm thì không hỏng lại.

**`plugins_data` — 109/1500.** Hai chỗ trong cùng một khối dữ liệu mang tên
`mimeTypes` nhưng **khác dạng**, và chính sự trùng tên đó làm lỗi sống lâu:

```
plugins[].mimeTypes   6342 đối tượng {type, suffixes, description, enabledPlugin}
mimeTypes (cấp trên)  1371 CHUỖI     "Portable Document Format~~application/pdf~~pdf"
```

Kiểu chỉ nhận đối tượng, nên một chuỗi làm **cả** `PluginsData` thất bại. Nay
nhận cả hai; chuỗi tách theo `description~~type~~suffixes` — cả 1371 giá trị
đều đúng hai dấu phân cách. Đây là **phân tích**, không phải bịa: ba trường nằm
sẵn trong chuỗi. `enabled_plugin` để `None` vì chuỗi không mang nó.

Sau bản vá: cả hai **100%**.

### Vì sao không ai thấy — và phần đáng nhớ hơn cả bản vá

`plugins_data_parseable` **có** tồn tại từ trước. Nó tính cờ `any_plugins` rồi
kết thúc bằng:

```rust
// Note: not all network samples have plugins — just verify parsing works
let _ = any_plugins;
```

Cờ bị **vứt đi**, kèm một chú thích hợp lý hoá **sai**: đo ra thì mẫu *có*
plugin — kiểu không khớp mới là nguyên nhân. Bài `audio_codecs_populated` ngay
bên trên thì kết bằng `assert!(any_codecs, ...)` và nó thật.

Một cờ tính rồi vứt, cộng một chú thích nghe hợp lý, là cách một lỗ hổng sống
sót qua mọi lượt chạy test.

### Thêm cổng chặn cả lớp lỗi này

`moi_truong_option_duoc_dien_o_ti_le_da_do` — khoá **hai chiều**. Sàn chặn hồi
quy; **trần** bắt ai sửa kiểu phải cập nhật bảng trong cùng lượt. Chính nó buộc
bản phát hành này sửa bảng: nó đỏ ở vế trần với *"plugins_data: 100.0% > tran
25.0% — truong nay TOT LEN"*.

Sàn đặt **gần chế độ hỏng**, không gần giá trị thường. Bản đầu đặt sàn `battery`
95,0 theo số đo 99,1% — nhưng phép đo đó chạy *có* ràng buộc OS còn bài test
chạy *không*, nên nó ra 93,3% và cổng đỏ oan ngay lượt đầu. Sàn 60 cách xa mọi
tỉ lệ quan sát được mà vẫn bắt được sự sụp đổ về ~0%.

`hai_truong_sua_o_023_cho_gia_tri_dung_chu_khong_chi_some` canh phần còn lại:
`Some` và *đúng* là hai chuyện khác nhau. Một bộ giải mã chấp nhận mọi thứ rồi
trả giá trị rỗng cũng cho 100% `Some`.

## 0.2.2 — 2026-09-11

Một bản sửa lỗi. **Đầu ra đổi so với 0.2.1** ở cùng một seed — 2,58% hồ sơ xin
Chrome trước đây nhận về user agent của họ khác, nay không còn. Không có thay
đổi API: không một mục `pub` nào đổi chữ ký.

### Sửa lỗi

**`.browser()` không tới được `navigator.userAgent`.** Xin `.browser(Chrome)`
và nhận về UA của Safari, Firefox, Edge, hoặc một crawler tự khai. Đo 2000 seed
mỗi OS trên 0.2.1:

```
bot-compatible            72/6000   1,20%
safari                    33/6000   0,55%
edge                      24/6000   0,40%
firefox                   13/6000   0,22%
chrome-thieu-duoi-Safari  13/6000   0,22%
                         ---------
                         155/6000   2,58%   ->  0/6000 sau bản sửa
```

Nguyên văn một ca mỗi loại:

```
...(KHTML, like Gecko; compatible; pageburst) Chrome/144.0.7559.132 ...
...AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.3 Safari/605.1.15
...(Windows NT 10.0; Win64; x64; rv:147.0) Gecko/20100101 Firefox/147.0
```

Đây là **bản sao của lỗi đã sửa ở 0.2.0 cho `.os()`**, cho ràng buộc anh em:
khối kẹp thẳng hai nút UA nằm bên trong nhánh `os`, nên `.browser()` không được
hưởng nó và chỉ ghim nút `*BROWSER` rồi trông chờ nó lan xuống — đúng cơ chế mà
chú thích của chính bản vá đó tuyên bố là không đủ. Bản sửa nâng khối kẹp ra
ngoài và cho hai vị từ **giao nhau**.

Thêm `ua_khop_browser`, song song với `ua_khop_os`. Bốn bẫy trong dữ liệu, ghi
ở docstring: UA Chrome luôn chứa `like Gecko` nên không được nhận Firefox bằng
`Gecko`; Edge và Opera đều mang `Chrome/`; Safari thật có `Version/` và không
có `Chrome/`; `compatible;` trong ngoặc AppleWebKit là crawler tự khai.

### Ghi chú cho người đọc mã

Cổng canh đầu tiên viết cho bản sửa này là **cổng giả**: khẳng định gọi chính
vị từ mà ràng buộc dùng, nên một lượt phá hoại (cho vị từ luôn trả `true`) vẫn
xanh — đột biến làm rỗng cả bộ lọc lẫn khẳng định cùng lúc. Khẳng định hiện tại
viết thẳng bằng `contains` và đọc được độc lập với cài đặt.

## 0.2.1 — 2026-09-05

Nối tiếp 0.2.0. **Đầu ra đổi so với 0.2.0** ở cùng một seed — có chủ ý, xem
mục cuối. Không có thay đổi API: không một mục `pub` nào đổi chữ ký.

### Sửa lỗi

0.2.0 sửa `navigator.platform` cho khớp user agent. Việc đó làm lộ ra mâu
thuẫn ở tầng kế tiếp: trước đây `platform` cũng sai nên hai bên cùng sai và
không phép kiểm nào thấy gì. Ba đường dưới đây sửa tầng đó, mỗi đường một cơ
chế khác nhau.

**Lọc `userAgentData` theo CẢ `platform` LẪN `platformVersion`.** Trước chỉ lọc
`"platform":"X"`, nên một khối khai đúng `"platform":"Windows"` vẫn mang được
`"platformVersion":"10.0"` — chuỗi không Chrome nào gửi.

**Suy `architecture` từ renderer.** Hàm thuần, không phải bịa: Chrome trên
Apple Silicon luôn gửi `"arm"`, trên x86 luôn `"x86"`. Bộ dữ liệu Apify cào từ
traffic thật nên có cả giá trị mâu thuẫn — đo trên 600 hồ sơ Windows với GPU
không phải Apple: 3 khối khai `"arm"`, 3 khai `"x64"` (còn không phải giá trị
UA-CH hợp lệ). **Chỉ ghi đè khi đã khai và mâu thuẫn**; `None` là "không khai",
mà không khai thì không có gì mâu thuẫn.

**Chuẩn hoá định dạng `platformVersion`**, không sửa giá trị: `"10_15_7"` →
`"10.15.7"` (gạch dưới là dạng của chuỗi UA, không phải của UA-CH), `"10.0"` →
`"10.0.0"`.

Đo trên 1500 hồ sơ, 22 phép kiểm nhất quán:

```
UA-CH architecture matches GPU          12,1%  ->  0,5%
UA-CH platformVersion matches platform  11,9%  ->  7,4%
diem trung binh                          94,4  ->  95,2
```

### Cố ý KHÔNG sửa

**Chuỗi rỗng** (73/500 hồ sơ macOS, 5/500 Windows). Đặt một giá trị vào chỗ
rỗng là **bịa**, không phải suy. Chrome thật có gửi chuỗi rỗng.

**Linux gửi phiên bản kernel** (`6.8.0` 25/500, cùng `6.11.0`, `6.14.0`). Luật
nội bộ của chúng tôi nói Linux phải rỗng, nhưng **chưa ai xác minh Chrome trên
Linux thật sự gửi rỗng**. Đây có thể là luật sai chứ không phải dữ liệu sai, và
sửa dữ liệu theo một luật chưa kiểm thì phải chắc luật đúng trước. Phép kiểm
mới cố ý bỏ qua Linux và ghi rõ lý do đó tại chỗ.

### Đầu ra đổi

Cùng seed, cùng phiên bản dữ liệu, `0.2.1` sinh ra hồ sơ khác `0.2.0` ở
`userAgentData.architecture` và `userAgentData.platformVersion`. Nếu bạn đang
ghim hồ sơ theo seed thì sinh lại và ghim lại.

## 0.2.0 — 2026-09-04

Bản sửa lỗi đúng đắn. **Đầu ra đổi so với 0.1.0** ở cùng một seed — có chủ ý,
xem mục cuối.

### Sửa lỗi

**`seeded()` không tái lập được qua các lần chạy.** `CptNode::Object` dùng
`HashMap`, mà `leaf_probabilities()` duyệt nó rồi `sample_from_probs` cộng dồn
trọng số *theo thứ tự* cho tới khi vượt ngưỡng. `HashMap` ngẫu nhiên hoá thứ
tự duyệt theo tiến trình, nên cùng seed cho hai kết quả khác nhau ở hai lần
chạy. Băm navigator của 1500 hồ sơ qua ba tiến trình cho ba giá trị khác nhau;
sau khi sửa cho một giá trị.

Lỗi thưa nên một phép thử 30 seed không thấy gì, và `examples/seeded_batch.rs`
in `Match: true` vì nó so trong *cùng một* tiến trình.

**Ràng buộc `.os()` không tới được user agent.** Mạng header nhận ràng buộc
còn mạng fingerprint không, và `operating_system` đọc từ mạng này trong khi
`userAgent` đọc từ mạng kia. Đo 2000 seed mỗi OS:

```
os=Windows  UA khong phai Windows:  930/2000 (46,5%)  ->  0/2000
os=macOS    UA khong phai macOS:   1486/2000 (74,3%)  ->  0/2000
os=Linux    UA khong phai Linux:   1715/2000 (85,8%)  ->  0/2000
```

**`navigator.platform` mâu thuẫn với user agent.** Bộ dữ liệu Apify cào từ
traffic thật, trong đó có máy đang spoof hỏng — 34/83 UA Windows có nhánh CPT
`platform` lấn sang OS khác, một trong số đó là `{"Linux x86_64": 1.0}`. Lọc
tập giá trị theo UA đã chốt, và suy ra khi CPT không cấp được giá trị nào hợp
lệ. Trượt phép kiểm "platform khớp OS" giảm từ 5,8% xuống 0,1% trên 1500 hồ sơ.

### Thay đổi phá vỡ API

`CptNode::Object` và `CptNode::get_deeper()` đổi từ `HashMap` sang `BTreeMap`.
Sửa bằng *kiểu* chứ không bằng một lời gọi `sort` trước khi bốc: lời gọi đó sẽ
bị quên ở đường code kế tiếp, còn kiểu thì không thể quên.

`sample_ancestral_with_evidence` nhận thêm tham số `filters`.

### Đầu ra đổi

Cùng một seed cho hồ sơ khác 0.1.0. Không tránh được: sửa thứ tự duyệt là đổi
giá trị được bốc. Hồ sơ sinh bằng 0.1.0 vốn *đã* không tái lập được, nên không
có gì để giữ tương thích.

### Còn lại

`userAgentData` (UA-CH) vẫn có thể mâu thuẫn với `platform` — khối đó là JSON
lồng nên cần parse và dựng lại, chưa làm.

## 0.1.0 — 2026-04-05

Bản phát hành đầu.
