$ErrorActionPreference='Stop'
$root=Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$fx=Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319'
$wpf=Join-Path $fx 'WPF'
$out=Join-Path $root 'prototypes\pocold\target\release\PoCol-Wallet.exe'
New-Item -ItemType Directory -Path (Split-Path $out -Parent) -Force | Out-Null
$CompilerArgs=@('/nologo','/target:winexe','/platform:x64',"/out:$out",'/reference:System.dll','/reference:System.Core.dll','/reference:System.Web.Extensions.dll','/reference:System.Xaml.dll',"/reference:$wpf\WindowsBase.dll","/reference:$wpf\PresentationCore.dll","/reference:$wpf\PresentationFramework.dll","/resource:$root\prototypes\pocol-windows\Wallet.xaml,Wallet.xaml","$root\prototypes\pocol-windows\Wallet.cs")
& "$fx\csc.exe" @CompilerArgs
if($LASTEXITCODE -ne 0){throw 'Wallet compilation failed'}
& $out --preview (Join-Path $root 'wallet-preview.png')
