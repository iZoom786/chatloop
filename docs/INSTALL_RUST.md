# How to Install Rust on Windows

## Option 1: Using rustup (Recommended)

1. **Download and run rustup-init.exe**:
   - Go to: https://rustup.rs/
   - Or download directly: https://rustup.rs/dist/x86_64-pc-windows-msvc/rustup-init.exe

2. **Run the installer**:
   - Double-click `rustup-init.exe`
   - It will open a command prompt
   - Press `1` or just `Enter` to continue with default installation
   - Press `Enter` to accept default settings

3. **Restart your terminal** (close and reopen PowerShell)

4. **Verify installation**:
   ```powershell
   rustc --version
   cargo --version
   ```

## Option 2: Using Chocolatey

If you have Chocolatey installed:

```powershell
choco install rust -y
```

## Option 3: Using Winget (Windows 10/11)

```powershell
winget install Rustlang.Rustup
```

## After Installation

Once Rust is installed, go to your ChatLoop directory and run:

```powershell
cd E:\chatloop
.\build.ps1 build
```

This will take several minutes on the first build as it downloads and compiles all dependencies.

## Troubleshooting

### If cargo still not found after installation:

1. **Close all terminals and reopen**
2. **Verify PATH includes Rust**:
   ```powershell
   $env:PATH -split ';' | Select-String rust
   ```
3. **If not in PATH, add it manually**:
   ```powershell
   $env:Path += ";C:\Users\$env:USERNAME\.cargo\bin"
   ```

### For permanent PATH addition:

1. Open "Environment Variables" in Windows
2. Edit "Path" under User variables
3. Add: `C:\Users\YourUsername\.cargo\bin`
4. Restart terminal

## Quick Install Command (One-liner)

In PowerShell, run:

```powershell
# Download and run rustup
Invoke-WebRequest -Uri https://rustup.rs/dist/x86_64-pc-windows-msvc/rustup-init.exe -OutFile rustup-init.exe
.\rustup-init.exe -y --default-toolchain stable

# Refresh environment variables
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","User") + ";" + [System.Environment]::GetEnvironmentVariable("Path","Machine")

# Verify
rustc --version
cargo --version
```

Then try building again:

```powershell
cd E:\chatloop
.\build.ps1 build
```
