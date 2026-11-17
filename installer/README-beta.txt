RniType 0.3.8 — beta สำหรับทดสอบ
================================

วิธีใช้
1. เปิด RniType.exe (จะเห็นหน้าต่าง console แสดง log — ปิดได้หลังทดสอบ)
2. เสียบจอย Xbox/DualSense
3. ไอคอนอยู่ที่ถาดระบบ (system tray) — คลิกซ้าย = Settings, คลิกขวา = Pause / Settings / Quit
4. เปิด Notepad แล้วพิมพ์ด้วยจอย:
   - LS เลือกกลุ่ม (วงใน) · RS เลือกตัวอักษร (วงนอก) · ปล่อย RS = พิมพ์
   - A = Space · B = Backspace · X = Enter · Y = Tab (ค้าง = Esc)
   - D-pad ← = วงคำสั่ง · LB ค้าง = quick bar · R3 = caret nav
5. Settings
   - General: Pause, exclusion list, CPU/RAM, WPM
   - Languages: เปิด/ปิดภาษา (EN/TH built-in, RU/EL สร้างจาก Windows layout)
   - Keymap: แก้รายช่อง (Base / Layer 2) + Reset
   - Practice: วัด error rate จริง (พิมพ์ประโยคอ้างอิง = 0 error, ผลเก็บที่ %APPDATA%\RniType\bench.jsonl)
   - HUD: ขนาดตัวหนังสือ/วง/ฟอนต์

ข้อจำกัด beta
- IME (ญี่ปุ่น/จีน/เกาหลี) ยังไม่รองรับ — จะทำในเวอร์ชันถัดไป
- ยังไม่ได้ code-sign → Windows SmartScreen อาจเตือน "Unknown publisher" (More info → Run anyway)
- console window เปิดไว้เพื่อให้เห็น log; รุ่นขายจริงจะซ่อน

ถอนการติดตั้ง
- Settings → Apps → RniType → Uninstall (ถ้าใช้ installer)
- หรือลบโฟลเดอร์ที่แตกไฟล์ (ถ้าใช้ portable)

config อยู่ที่ %APPDATA%\RniType\config.json (ลบได้เพื่อ reset)
