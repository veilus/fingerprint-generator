# Changelog

## 0.3.0 — 2026-09-19

**THAY ĐỔI PHÁ VỠ.** `NavigatorFingerprint::max_touch_points` đổi từ
`Option<u8>` sang `Option<u16>`. Người dùng đang khớp kiểu tường minh (`let x:
Option<u8> = fp.navigator.max_touch_points;`) phải sửa một dòng; người dùng chỉ
đọc giá trị thì không phải làm gì.

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

### Thứ bản này KHÔNG sửa

`hardwareConcurrency` mang **cùng lớp lỗi và nặng hơn 150 lần**: 4 giá trị của
nó vượt `u8` (384, 448, 512, 640), chiếm **26,4%** khối lượng xác suất toàn
mạng, và 127/479 phân phối có điều kiện mất hơn một nửa khối lượng. Nó còn tệ
hơn ở chỗ mất thành `.unwrap_or(4)` — một con số **trông hợp lý** — chứ không
thành `None`.

Không gộp vào bản này vì nó đặt ra một câu hỏi mà `maxTouchPoints` không đặt:
`384` nhân cho một UA **Macintosh** là giá trị không tồn tại trên đời, nên nới
kiểu để phát nó ra có thể tạo một dấu vân tay MỚI thay vì sửa một dấu vân tay
sai. Đó là quyết định về dữ liệu, không phải về kiểu.

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
