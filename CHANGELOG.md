# Changelog

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
