# Empacota o binário já compilado por scripts/build-rpm.sh. A compilação fica
# fora do rpmbuild para não exigir acesso à rede nem crates vendorizados.
%global debug_package %{nil}
%global app_id io.github.rodolfoaltenrath.Redoma

Name:           redoma
Version:        %{redoma_version}
Release:        1%{?dist}
Summary:        App oficial Certisign no Linux, isolado em contêiner Waydroid
License:        GPL-3.0-or-later
URL:            https://github.com/rodolfoaltenrath/redoma-linux
Source0:        redoma
Source1:        redoma.desktop
Source2:        %{app_id}.svg
Source3:        LICENSE
Source4:        README.md
Source5:        SECURITY.md
ExclusiveArch:  x86_64

# O assistente instala a imagem Android e aplica o ajuste de compatibilidade;
# o Waydroid e o pkexec vêm como dependências do pacote.
Requires:       waydroid
Requires:       polkit
Requires:       hicolor-icon-theme

%description
Executa o aplicativo oficial Certisign num contêiner Android (Waydroid), numa
janela própria em formato de celular. Na primeira execução, um assistente
gráfico prepara o ambiente Android e guia a instalação do app pela Play Store.

Projeto independente, sem vínculo com a Certisign. Cadastro, e-token e
autorização acontecem exclusivamente no aplicativo oficial.

%prep

%build

%install
install -Dpm 0755 %{SOURCE0} %{buildroot}%{_bindir}/redoma
install -Dpm 0644 %{SOURCE1} %{buildroot}%{_datadir}/applications/redoma.desktop
install -Dpm 0644 %{SOURCE2} %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/%{app_id}.svg
install -Dpm 0644 %{SOURCE3} %{buildroot}%{_licensedir}/%{name}/LICENSE
install -Dpm 0644 %{SOURCE4} %{buildroot}%{_docdir}/%{name}/README.md
install -Dpm 0644 %{SOURCE5} %{buildroot}%{_docdir}/%{name}/SECURITY.md

%files
%license %{_licensedir}/%{name}/LICENSE
%doc %{_docdir}/%{name}/README.md
%doc %{_docdir}/%{name}/SECURITY.md
%{_bindir}/redoma
%{_datadir}/applications/redoma.desktop
%{_datadir}/icons/hicolor/scalable/apps/%{app_id}.svg

%changelog
* Sat Oct 24 2026 Djonatan Altenrath <rodolfoaltenrath@gmail.com> - 0.1.0-1
- Primeiro pacote: assistente gráfico, launcher e ações administrativas via pkexec.
