Name:           openvidu-sync
Version:        0.3.0
Release:        leoyu.0.3.0%{?dist}
Summary:        OpenVidu recording sync tool

License:        MIT
URL:            https://github.com/leoyu0607/Openvidu-record-sync
# 這些檔案已經放在 /root/rpmbuild/SOURCES
Source0:        openvidu_sync
Source1:        Config.json
Source2:        openvidu-sync.service
Source3:        openvidu-sync.timer

BuildArch:      x86_64
Requires:	rsync

%description
A small tool to sync OpenVidu recordings to NAS using rsync.

%prep
# 已經是編譯好的 binary，這裡不用做事

%build
# 也不需要 build，因為我們直接打包已編譯檔案

%install
rm -rf %{buildroot}

# 安裝 binary
install -d %{buildroot}/opt/openvidu-sync
install -m 0755 %{SOURCE0} %{buildroot}/opt/openvidu-sync/openvidu_sync

# 安裝預設設定檔（給管理員修改）
install -d %{buildroot}/opt/openvidu-sync
install -m 0644 %{SOURCE1} %{buildroot}/opt/openvidu-sync/Config.json

# 安裝 systemd unit
install -d %{buildroot}/usr/lib/systemd/system
install -m 0644 %{SOURCE2} %{buildroot}/usr/lib/systemd/system/openvidu-sync.service
install -m 0644 %{SOURCE3} %{buildroot}/usr/lib/systemd/system/openvidu-sync.timer

%files
%license
%doc
/opt/openvidu-sync/openvidu_sync
%config(noreplace) /opt/openvidu-sync/Config.json
/usr/lib/systemd/system/openvidu-sync.service
/usr/lib/systemd/system/openvidu-sync.timer

%post
# 安裝後重新載入 systemd，並啟用 timer（可依需求決定要不要自動 enable）
if [ $1 -eq 1 ] ; then
    /bin/systemctl daemon-reload >/dev/null 2>&1 || :
    #/bin/systemctl enable --now openvidu-sync.timer >/dev/null 2>&1 || :
fi

%preun
# 移除前停用 timer
if [ $1 -eq 0 ] ; then
    /bin/systemctl stop openvidu-sync.timer >/dev/null 2>&1 || :
    /bin/systemctl stop openvidu-sync.service >/dev/null 2>&1 || :
    /bin/systemctl disable --now openvidu-sync.timer >/dev/null 2>&1 || :
fi

%postun
/bin/systemctl daemon-reload >/dev/null 2>&1 || :

