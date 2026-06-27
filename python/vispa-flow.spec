# -*- mode: python ; coding: utf-8 -*-
# PyInstaller spec for Vispa Flow Python sidecar
# Build: pyinstaller vispa-flow.spec --distpath binaries

a = Analysis(
    ['main.py'],
    pathex=[],
    binaries=[],
    datas=[
        # Bundle faster-whisper models directory if pre-downloaded
        # ('../models/*', 'models'),
    ],
    hiddenimports=[
        'faster_whisper',
        'faster_whisper.transcribe',
        'faster_whisper.utils',
        'sounddevice',
        'sounddevice._sounddevice_data',
        'numpy',
        'numpy.core._methods',
        'numpy.lib.format',
        'silero_vad',
        'psutil',
        'openai',
        'ctypes',
        'json',
        'logging',
    ],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[
        'tkinter',
        'matplotlib',
        'jupyter',
        'IPython',
        'notebook',
    ],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=None,
    noarchive=False,
)

pyz = PYZ(a.pure, a.zipped_data, cipher=None)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.zipfiles,
    a.datas,
    [],
    name='vispa-python',
    debug=False,
    bootloader_ignore_signals=False,
    strip=True,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=True,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch='x86_64',
    codesign_identity=None,
    entitlements_file=None,
)
