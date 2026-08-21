# 📖 Dokumentasi Visual Antarmuka (UI) Wino

Dokumentasi lengkap antarmuka pengguna (**User Interface / GUI**) untuk aplikasi **Wino — Rust-Native Windows Debloater, Optimizer & Memory Suite**.

Semua aset gambar tangkapan layar (**screenshots**) pada dokumen ini tersimpan secara lokal pada folder [`docs/screenshots/`](./screenshots/) dan tidak bergantung pada hosting eksternal maupun GitHub.

---

## 📑 Daftar Isi Menu

1. [⊞ Dashboard (Sistem Telemetri & Kesehatan)](#1--dashboard-system-telemetry--health)
2. [⚡ Memory Engine (Manajemen Tekanan RAM)](#2--memory-engine-ram-management)
3. [≡ Process Manager (Inspektor Proses Aktif)](#3--process-manager)
4. [🧹 Windows Debloater (Pembersih Paket Bloatware)](#4--windows-debloater--optimizer)
5. [🚀 Startup Applications (Optimasi Boot & Startup)](#5--startup-applications)
6. [⚙ Windows Services (Manajemen Layanan Windows)](#6--windows-services)
7. [🛡 Privacy Center (Pusat Privasi & Telemetri)](#7--privacy-center)
8. [🗑 Storage Cleaner (Pembersih File Sementara & Sampah)](#8--storage-cleaner)
9. [🎮 Gaming Profile (Mode Gaming & Latensi Rendah)](#9--gaming-profile)
10. [✚ Windows Health (Diagnostik SFC & DISM)](#10--windows-health-diagnostics)
11. [↺ Restore & Safety Snapshots (Titik Pemulihan & Rollback)](#11--restore--safety-snapshots)
12. [📋 Audit & Event Logs (Catatan Riwayat Eksekusi)](#12--audit--event-logs)
13. [🔧 Settings & Preferences (Pengaturan Tema & Hak Akses)](#13--settings--preferences)

---

## 1. ⊞ Dashboard (System Telemetry & Health)

![System Dashboard](./screenshots/01_dashboard.png)

### 📌 Fungsi Utama
Halaman utama yang menampilkan ringkasan telemetri perangkat keras secara *real-time* dan status kesehatan sistem operasi Windows secara keseluruhan.

### 🔍 Fitur & Komponen UI
- **Grafik Penggunaan CPU**: Menampilkan beban kerja prosesor secara langsung beserta jumlah *logical core*.
- **System Health & Recommendations**: Sistem penilaian otomatis (`EXCELLENT`, `GOOD`, `ATTENTION`, `WARNING`, `CRITICAL`) beserta saran tindakan proaktif jika terdapat fitur keamanan atau sistem yang belum optimal.
- **Host Information**: Ringkasan versi Windows (Win 10/11, Build version), kartu grafis / GPU Adapter aktif, serta jumlah proses yang sedang berjalan.
- **Footer Status Bar**: Indikator status privileges (`Admin` / `User`), beban CPU/RAM, dan *live event ticker*.

---

## 2. ⚡ Memory Engine (RAM Management)

![Memory Engine](./screenshots/02_memory_engine.png)

### 📌 Fungsi Utama
Analisis mendalam konsumsi RAM fisik, cache sistem, commit charge, dan mitigasi tekanan memori (*memory pressure*) tanpa *placebo*.

### 🔍 Fitur & Komponen UI
- **Physical Memory Utilization Bar**: Visualisasi penggunaan RAM aktif vs memori yang tersedia (*Available Memory*) dan Standby Cache.
- **Committed Memory**: Memantau batas alokasi virtual memory yang dijamin oleh sistem / paging file.
- **Memory Pressure Badge**: Klasifikasi tingkat tekanan RAM (`LOW`, `MODERATE`, `HIGH`, `CRITICAL`).
- **Optimize Memory Now**: Mengeksekusi pemangkasan *working set* proses latar belakang yang tidak aktif menggunakan API native Win32 `EmptyWorkingSet`.
- **Simulate (Dry-Run)**: Menghitung estimasi RAM yang dapat dibebaskan tanpa melakukan modifikasi langsung.

---

## 3. ≡ Process Manager

![Process Manager](./screenshots/03_processes.png)

### 📌 Fungsi Utama
Inspektor proses yang berjalan pada sistem dengan verifikasi tanda tangan digital (*Authenticode Digital Signatures*) dan manajemen pemakaian memori per proses.

### 🔍 Fitur & Komponen UI
- **Search Bar**: Pencarian cepat instan berdasarkan nama proses atau PID.
- **Kolom Detail**: Menampilkan Nama Proses, PID, Working Set (RAM), dan Status Verifikasi Penerbit (`Verified Signed` / `Unverified`).
- **End Task Action**: Menghentikan proses yang membebani memori secara aman.
- **Proteksi Kernel**: Proses esensial sistem Windows terlindungi otomatis dari penghentian yang tidak disengaja.

---

## 4. 🧹 Windows Debloater & Optimizer

![Windows Debloater & Optimizer](./screenshots/04_debloat.png)

### 📌 Fungsi Utama
Menghapus bloatware bawaan Windows, aplikasi promosi OEM, pelacak iklan, serta menonaktifkan fitur latar belakang yang memboroskan daya dan bandwidth.

### 🔍 Fitur & Komponen UI
- **Preset Pilihan Cepat**:
  - `Safe (6 rules)`: Rekomendasi untuk semua pengguna (bebas risiko & 100% *reversible*).
  - `Balanced (11 rules)`: Termasuk Safe + mematikan Windows 11 Widgets (hemat ~200MB RAM), Copilot, Edge startup boost.
  - `Aggressive (15 rules)`: Pembersihan mendalam aplikasi OEM (TikTok, Disney, Spotify stubs), sensor lokasi, dsb.
- **Status Indikator**: Menandai aturan yang sudah `Optimized` atau `Default`.
- **Filter View & Search**: Menyaring daftar aturan berdasarkan kategori atau kata kunci.
- **Dry-Run & Konfirmasi Modal**: Pratinjau sebelum eksekusi dan dialog konfirmasi untuk preset berisiko lebih tinggi.

---

## 5. 🚀 Startup Applications

![Startup Applications](./screenshots/05_startup_apps.png)

### 📌 Fungsi Utama
Memindai dan mengelola program yang otomatis berjalan saat komputer menyala untuk mempercepat waktu *boot* dan meringankan beban *idle*.

### 🔍 Fitur & Komponen UI
- **Pendeteksian Multi-Lokasi**: Memindai Registry `HKCU\Run`, `HKLM\Run`, serta folder Startup pengguna dan publik.
- **Startup Impact Rating**: Memberikan bobot pengaruh terhadap waktu boot (`High Impact`, `Medium Impact`, `Low Impact`).
- **Disable / Enable Toggle**: Mematikan aplikasi startup yang tidak diperlukan dengan sekali klik tanpa merusak instalasi aplikasi aslinya.

---

## 6. ⚙ Windows Services

![Windows Services](./screenshots/06_services.png)

### 📌 Fungsi Utama
Memeriksa dan mengatur tipe *startup* layanan latar belakang Windows dengan klasifikasi keamanan yang ketat.

### 🔍 Fitur & Komponen UI
- **Klasifikasi Keamanan**:
  - `Safe to change` (Hijau): Layanan non-esensial (misal: Telemetri DiagTrack, MapsBroker).
  - `Optional` (Kuning): Layanan yang aman disesuaikan tergantung kebutuhan pengguna.
  - `Do not touch` (Merah): Layanan inti sistem yang dilindungi oleh mesin safety Wino.
- **Aksi Cepat**: Mengubah mode startup layanan menjadi `Demand / Manual` atau `Disabled`.
- **Live Search**: Pencarian instan di antara ratusan service Windows yang terpasang.

---

## 7. 🛡 Privacy Center

![Privacy Center](./screenshots/07_privacy_center.png)

### 📌 Fungsi Utama
Mengendalikan pengaturan privasi Windows, telemetri diagnostik, riwayat aktivitas, dan pelacakan iklan pengguna.

### 🔍 Fitur & Komponen UI
- **Advertising ID**: Memblokir identifikasi profil pengguna untuk iklan lintas aplikasi.
- **Activity History Collection**: Mencegah Windows mencatat riwayat pembukaan file dan navigasi aplikasi ke cloud.
- **Tailored Experiences & Feedback**: Mematikan penawaran rekomendasi dan kuesioner otomatis Microsoft.
- **Inking & Typing Personalization**: Memastikan data ketikan dan penulisan tetap berada di perangkat lokal.
- **Badge Status & Tombol Revert**: Menampilkan status `Protected` dan opsi untuk mengembalikan pengaturan kapan saja.

---

## 8. 🗑 Storage Cleaner

![Storage Cleaner](./screenshots/08_storage_cleaner.png)

### 📌 Fungsi Utama
Pembersihan aman file sampah sementara, *crash dumps*, cache instalasi, dan cache shader GPU tanpa menghapus file pribadi pengguna.

### 🔍 Fitur & Komponen UI
- **Kalkulator Ruang Terpulihkan**: Menghitung total GB/MB yang dapat dibebaskan sebelum tindakan dilakukan.
- **Target Aman**:
  - `User Temporary Files` (`%TEMP%` & `C:\Windows\Temp`)
  - `DirectX / GPU Shader Cache`
  - `Windows Delivery Optimization Cache`
  - `Crash Dumps & Error Reports`
- **Filter Umur Berkas**: Dilengkapi aturan keamanan umur file agar tidak menghapus file sementara yang sedang aktif digunakan program.

---

## 9. 🎮 Gaming Profile

![Gaming Profile](./screenshots/09_gaming_profile.png)

### 📌 Fungsi Utama
Mengoptimalkan sistem operasi untuk sesi bermain game dengan memprioritaskan alokasi CPU/GPU dan meminimalkan latensi latar belakang.

### 🔍 Fitur & Komponen UI
- **Windows Game Mode**: Memastikan penjadwalan proses game mendapatkan prioritas utama thread CPU dan GPU.
- **Optimize for Gaming**: Melakukan pembersihan RAM seketika sebelum meluncurkan game.
- **Prinsip Zero-Placebo**: Tidak melakukan modifikasi berbahaya pada registri atau *timer resolution hack* yang dapat menyebabkan *blue screen* (BSOD).

---

## 10. ✚ Windows Health Diagnostics

![Windows Health Diagnostics](./screenshots/10_windows_health.png)

### 📌 Fungsi Utama
Pemeriksaan integritas file sistem Windows dan perbaikan *component store* secara terpadu.

### 🔍 Fitur & Komponen UI
- **Overall System Health Rating**: Penilaian kesehatan sistem seketika.
- **SFC Scan Runner (`sfc /scannow`)**: Memeriksa dan memulihkan file sistem Windows yang rusak atau korup.
- **DISM Health Check**: Memeriksa dan memperbaiki citra komponen Windows (*Windows Component Store*).

---

## 11. ↺ Restore & Safety Snapshots

![Restore & Safety Snapshots](./screenshots/11_restore_points.png)

### 📌 Fungsi Utama
Manajemen titik pemulihan konfigurasi (*snapshots*) dan integrasi Windows System Restore Point untuk keamanan maksimal saat melakukan optimasi.

### 🔍 Fitur & Komponen UI
- **Otomatisasi Pre-Flight Snapshot**: Wino otomatis mencatat *snapshot* konfigurasi sebelum mengeksekusi optimasi batch.
- **Create Manual Snapshot**: Pembuatan snapshot konfigurasi kapan saja sesuai keinginan pengguna.
- **1-Click Restore**: Mengembalikan konfigurasi yang telah diubah ke kondisi semula secara instan dan aman.

---

## 12. 📋 Audit & Event Logs

![Audit & Event Logs](./screenshots/12_audit_logs.png)

### 📌 Fungsi Utama
Log sirkuler dalam memori yang mencatat seluruh operasi sistem, pemindaian, perubahan konfigurasi, dan peringatan secara kronologis.

### 🔍 Fitur & Komponen UI
- **Level Tag Berwarna**: Badge jelas untuk `INFO`, `WARN`, dan `ERROR`.
- **Target Kategori**: Pengelompokan log berdasarkan modul (`System`, `Security`, `Memory`, `Cleaner`, `Debloat`, dll.).
- **Clear Logs**: Tombol untuk mengosongkan riwayat log dari memori aplikasi.

---

## 13. 🔧 Settings & Preferences

![Settings & Preferences](./screenshots/13_settings.png)

### 📌 Fungsi Utama
Pengaturan tema antarmuka pengguna, preferensi aplikasi, dan pengelolaan hak akses eksekusi.

### 🔍 Fitur & Komponen UI
- **Appearance & Theme**: Pilihan tema Fluent `Dark` atau `Light`.
- **Execution Privileges**: Menampilkan status elevasi hak akses (`Standard User` atau `Administrator`).
- **Restart as Administrator**: Opsi 1-klik untuk memulai ulang Wino dengan hak akses Administrator ketika dibutuhkan untuk modifikasi registri tingkat dalam.
- **About Wino**: Informasi versi rilis, arsitektur native Rust, dan lisensi open source.

---

## 🛠 Cara Memperbarui Screenshot Secara Otomatis

Jika terdapat pembaruan tampilan antarmuka di masa mendatang, seluruh screenshot di atas dapat diperbarui secara otomatis menggunakan skrip lokal berikut:

```powershell
# Jalankan skrip otomasi capture screenshot
powershell -ExecutionPolicy Bypass -File scripts\capture.ps1
```

Skrip ini akan mengompilasi alat otomasi capture, menjalankan setiap tab secara berurutan, mengambil frame gambar beresolusi tinggi, dan menyimpannya langsung ke folder `docs/screenshots/` dalam format `.png`.
