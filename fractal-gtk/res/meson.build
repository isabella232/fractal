subdir('icons')

desktop_conf = configuration_data()
desktop_conf.set('icon', application_id)
i18n.merge_file ('desktop-file',
  type: 'desktop',
  input: configure_file(
    input: files('org.gnome.Fractal.desktop.in.in'),
    output: 'org.gnome.Fractal.desktop.in',
    configuration: desktop_conf
  ),
  output: '@0@.desktop'.format(application_id),
  po_dir: podir,
  install: true,
  install_dir: join_paths (datadir, 'applications')
)

appdata_conf = configuration_data()
appdata_conf.set('appid', application_id)
i18n.merge_file ('appdata-file',
  input: configure_file(
    input: files('org.gnome.Fractal.appdata.xml.in.in'),
    output: 'org.gnome.Fractal.appdata.xml.in',
    configuration: appdata_conf
  ),
  output: '@0@.appdata.xml'.format(application_id),
  po_dir: podir,
  install: true,
  install_dir: join_paths (datadir, 'metainfo')
)

install_data('org.gnome.Fractal.gschema.xml', install_dir: join_paths(get_option('datadir'), 'glib-2.0', 'schemas'))
