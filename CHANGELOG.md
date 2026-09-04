# Changelog

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
