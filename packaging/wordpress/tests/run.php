<?php
/**
 * Test fără WordPress: stub-uri minime pentru funcțiile WP folosite de Veriga_Runner, apoi rulează
 * binarul VERIGA real pe un server HTTP local.
 *
 *   VERIGA_BIN=target/debug/veriga VERIGA_SITE=http://127.0.0.1:8777/ php packaging/wordpress/tests/run.php
 */
define( 'ABSPATH', __DIR__ );
$GLOBALS['opts'] = $GLOBALS['trans'] = $GLOBALS['mails'] = array();
$tmp = sys_get_temp_dir() . '/vlc-test-' . getmypid();
@mkdir( $tmp . '/uploads', 0777, true );
function get_option( $k, $d = false ) { return $GLOBALS['opts'][ $k ] ?? $d; }
function update_option( $k, $v, $a = null ) { $GLOBALS['opts'][ $k ] = $v; return true; }
function delete_option( $k ) { unset( $GLOBALS['opts'][ $k ] ); }
function get_transient( $k ) { return $GLOBALS['trans'][ $k ] ?? false; }
function set_transient( $k, $v, $e = 0 ) { $GLOBALS['trans'][ $k ] = $v; return true; }
function delete_transient( $k ) { unset( $GLOBALS['trans'][ $k ] ); }
function wp_upload_dir( $t = null, $c = true ) { return array( 'basedir' => $GLOBALS['tmp'] . '/uploads' ); }
function trailingslashit( $s ) { return rtrim( $s, '/\\' ) . '/'; }
function wp_mkdir_p( $d ) { return @mkdir( $d, 0777, true ); }
function home_url( $p = '' ) { return getenv( 'VERIGA_SITE' ) ?: 'http://127.0.0.1:8777/'; }
function get_locale() { return 'ro_RO'; }
function get_temp_dir() { return sys_get_temp_dir() . '/'; }
function is_email( $e ) { return (bool) filter_var( $e, FILTER_VALIDATE_EMAIL ); }
function get_bloginfo( $k ) { return 'Site de test'; }
function wp_specialchars_decode( $s, $q = 0 ) { return $s; }
function admin_url( $p = '' ) { return 'https://exemplu.ro/wp-admin/' . $p; }
function __( $s, $d = '' ) { return $s; }
function _n( $a, $b, $n, $d = '' ) { return 1 === $n ? $a : $b; }
function is_multisite() { return false; }
function wp_mail( $to, $s, $b ) { $GLOBALS['mails'][] = compact( 'to', 's', 'b' ); return true; }
function wp_clear_scheduled_hook( $h ) { $GLOBALS['unscheduled'] = $h; }
$GLOBALS['tmp'] = $tmp;

require __DIR__ . '/../veriga-link-checker/includes/class-veriga-runner.php';

$fail = 0;
function ok( $cond, $msg ) { global $fail; echo ( $cond ? 'ok   : ' : 'FAIL : ' ) . $msg . "\n"; if ( ! $cond ) { $fail++; } }

$bin = realpath( getenv( 'VERIGA_BIN' ) ?: __DIR__ . '/../../../target/debug/veriga' );
ok( $bin && is_executable( $bin ), 'binarul VERIGA există: ' . $bin );

// validate_binary
ok( 'empty' === Veriga_Runner::validate_binary( '' ), 'cale goală respinsă' );
ok( 'not_absolute' === Veriga_Runner::validate_binary( 'veriga' ), 'cale relativă respinsă' );
ok( 'not_absolute' === Veriga_Runner::validate_binary( "/usr/bin/veriga\0x" ), 'octet NUL respins' );
ok( 'bad_name' === Veriga_Runner::validate_binary( '/bin/sh' ), 'nume care nu începe cu veriga respins' );
ok( 'bad_name' === Veriga_Runner::validate_binary( '/tmp/veriga; rm -rf /' ), 'nume cu metacaractere respins' );
ok( 'not_file' === Veriga_Runner::validate_binary( '/tmp/veriga-inexistent' ), 'fișier inexistent respins' );
file_put_contents( $tmp . '/veriga-noexec', "#!/bin/sh\n" ); chmod( $tmp . '/veriga-noexec', 0644 );
ok( 'not_executable' === Veriga_Runner::validate_binary( $tmp . '/veriga-noexec' ), 'neexecutabil respins' );
file_put_contents( $tmp . '/veriga-ww', "#!/bin/sh\n" ); chmod( $tmp . '/veriga-ww', 0777 );
ok( 'world_writable' === Veriga_Runner::validate_binary( $tmp . '/veriga-ww' ), 'world-writable respins' );
ok( '' === Veriga_Runner::validate_binary( $bin ), 'binarul real acceptat' );

// build_args: tipuri, fără shell, limite
$s = array_merge( Veriga_Runner::defaults(), array( 'binary' => $bin, 'max_urls' => 99999999, 'no_external' => 1, 'allow_priv' => 1 ) );
$a = Veriga_Runner::build_args( $s, 'http://x.test/', '/o/r.json', '/o/r.html', 'ro' );
ok( $a[0] === $bin && $a[1] === 'http://x.test/', 'argumentele încep cu binar + URL' );
ok( in_array( '1000000', $a, true ) && in_array( '--no-external', $a, true ) && in_array( '--allow-private', $a, true ), 'max_urls limitat, opțiuni adăugate' );
$a2 = Veriga_Runner::build_args( array_merge( $s, array( 'max_urls' => -5, 'no_external' => 0, 'allow_priv' => 0 ) ), 'http://x.test/', 'j', 'h', 'xx' );
ok( in_array( '100', $a2, true ) && ! in_array( '--allow-private', $a2, true ) && 'en' === $a2[ array_search( '--lang', $a2, true ) + 1 ], 'limită minimă, fără allow-private implicit, limbă implicită en' );

// rulare reală
update_option( Veriga_Runner::OPT, array_merge( Veriga_Runner::defaults(), array( 'binary' => $bin, 'allow_priv' => 1, 'no_external' => 1, 'emails' => 'a@exemplu.ro, nu-e-email, b@exemplu.ro' ) ) );
$st = Veriga_Runner::run();
ok( $st['ok'] && '' === $st['error'], 'rularea reală reușește (exit ' . ( $st['exit'] ?? '?' ) . ')' );
ok( 1 === ( $st['broken'] ?? -1 ), 'o legătură ruptă găsită' );
ok( ( $st['top'][0]['code'] ?? 0 ) === 404 && substr( $st['top'][0]['url'], -9 ) === 'nope.html' && '' !== $st['top'][0]['page'], 'detalii: 404, URL, pagina-sursă' );
ok( ! empty( $st['has_html'] ) && ! empty( $st['has_json'] ), 'rapoartele HTML și JSON există' );
$dir = Veriga_Runner::reports_dir( false );
ok( is_file( $dir . '/index.php' ) && is_file( $dir . '/.htaccess' ) && preg_match( '#/veriga-lc-[a-f0-9]{24}$#', $dir ), 'director de rapoarte cu nume aleator și protecții' );
ok( 1 === count( $GLOBALS['mails'] ) && $GLOBALS['mails'][0]['to'] === array( 'a@exemplu.ro', 'b@exemplu.ro' ), 'e-mail către adresele valide, cea invalidă ignorată' );
ok( false !== strpos( $GLOBALS['mails'][0]['s'], '1 broken link' ) && false !== strpos( $GLOBALS['mails'][0]['b'], 'nope.html' ), 'subiectul și corpul e-mailului' );
ok( false === get_transient( Veriga_Runner::LOCK ), 'blocarea eliberată după rulare' );

// concurență
set_transient( Veriga_Runner::LOCK, time() );
ok( 'busy' === Veriga_Runner::run()['error'], 'rulare simultană refuzată' );
delete_transient( Veriga_Runner::LOCK );

// site curat: fără e-mail când notify=broken
$GLOBALS['mails'] = array();
$clean = $tmp . '/clean'; mkdir( $clean ); file_put_contents( $clean . '/index.html', '<a href="index.html">ok</a>' );
$srv = proc_open( array( 'python3', '-m', 'http.server', '8778', '--bind', '127.0.0.1', '-d', $clean ), array( 1 => array( 'file', '/dev/null', 'w' ), 2 => array( 'file', '/dev/null', 'w' ) ), $p );
usleep( 800000 );
putenv( 'VERIGA_SITE=http://127.0.0.1:8778/' );
$st = Veriga_Runner::run();
ok( $st['ok'] && 0 === $st['broken'] && 0 === count( $GLOBALS['mails'] ), 'site curat: 0 rupte, niciun e-mail' );
proc_terminate( $srv );

// binar defect: exit 1 → eșec + e-mail de eroare
file_put_contents( $tmp . '/veriga-fail', "#!/bin/sh\necho boom >&2\nexit 1\n" ); chmod( $tmp . '/veriga-fail', 0755 );
update_option( Veriga_Runner::OPT, array_merge( Veriga_Runner::settings(), array( 'binary' => $tmp . '/veriga-fail' ) ) );
$st = Veriga_Runner::run();
ok( ! $st['ok'] && 'exit_1' === $st['error'] && false !== strpos( $st['log'], 'boom' ), 'cod de ieșire 1 = eșec, jurnal păstrat' );
ok( 1 === count( $GLOBALS['mails'] ) && false !== strpos( $GLOBALS['mails'][0]['s'], 'failed' ), 'e-mail de eșec trimis' );

// dezinstalare
Veriga_Runner::delete_reports_dir();
ok( ! is_dir( $dir ), 'directorul de rapoarte șters la dezinstalare' );
Veriga_Runner::unschedule();
ok( Veriga_Runner::HOOK === ( $GLOBALS['unscheduled'] ?? '' ), 'evenimentul cron eliminat' );

echo $fail ? "\n$fail TESTE EȘUATE\n" : "\nTOATE TESTELE PLUGINULUI AU TRECUT\n";
exit( $fail ? 1 : 0 );
