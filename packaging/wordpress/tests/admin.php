<?php
/** Test de fum pentru Veriga_Admin: randare ambele file cu date ostile în starea salvată; verifică escaparea. */
define( 'ABSPATH', __DIR__ );
define( 'VERIGA_LC_VERSION', '1.3.1' );
define( 'VERIGA_LC_FILE', realpath( __DIR__ . '/../veriga-link-checker' ) . '/veriga-link-checker.php' );
define( 'VERIGA_LC_DIR', dirname( VERIGA_LC_FILE ) . '/' );
$GLOBALS['opts'] = array();
function add_action() {} function add_filter() {} function add_management_page() {}
function plugin_basename( $f ) { return basename( dirname( $f ) ) . '/' . basename( $f ); }
function esc_html( $s ) { return htmlspecialchars( (string) $s, ENT_QUOTES ); }
function esc_attr( $s ) { return htmlspecialchars( (string) $s, ENT_QUOTES ); }
function esc_url( $s ) { return htmlspecialchars( (string) $s, ENT_QUOTES ); }
function esc_html__( $s, $d = '' ) { return esc_html( $s ); }
function __( $s, $d = '' ) { return $s; }
function _n( $a, $b, $n, $d = '' ) { return 1 === $n ? $a : $b; }
function current_user_can( $c ) { return true; }
function is_multisite() { return false; }
function admin_url( $p = '' ) { return 'https://x.test/wp-admin/' . $p; }
function home_url( $p = '' ) { return 'https://x.test' . $p; }
function add_query_arg( $a, $u ) { return $u . '?' . http_build_query( $a ); }
function plugins_url( $p, $f ) { return 'https://x.test/wp-content/plugins/veriga-link-checker/' . $p; }
function wp_nonce_field( $a ) { echo '<input type="hidden" name="_wpnonce" value="n">'; }
function wp_nonce_url( $u, $a ) { return $u . '&_wpnonce=n'; }
function selected( $a, $b, $e = true ) { return $a === $b ? ' selected' : ''; }
function checked( $a, $b, $e = true ) { return $a === $b ? ' checked' : ''; }
function submit_button( $t = '', $ty = '', $n = '', $w = true, $a = array() ) { echo '<button>' . esc_html( $t ) . '</button>'; }
function get_option( $k, $d = false ) { return $GLOBALS['opts'][ $k ] ?? $d; }
function update_option( $k, $v, $a = null ) { $GLOBALS['opts'][ $k ] = $v; return true; }
function get_transient( $k ) { return false; }
function wp_next_scheduled( $h ) { return time() + 3600; }
function wp_date( $f, $t ) { return gmdate( $f, $t ); }
require VERIGA_LC_DIR . 'includes/class-veriga-runner.php';
require VERIGA_LC_DIR . 'includes/class-veriga-admin.php';
$fail = 0;
function ok( $c, $m ) { global $fail; echo ( $c ? 'ok   : ' : 'FAIL : ' ) . $m . "\n"; if ( ! $c ) { $fail++; } }

$evil = '"><script>alert(1)</script>';
update_option( Veriga_Runner::OPT, array( 'binary' => $evil, 'emails' => $evil ) );
update_option( Veriga_Runner::STATE, array( 'time' => 1, 'ok' => true, 'error' => '', 'broken' => 3, 'truncated' => true, 'has_html' => true, 'has_json' => true,
	'top' => array( array( 'url' => 'http://a/' . $evil, 'status' => 'NotFound', 'code' => 404, 'page' => 'http://p/' . $evil ) ) ) );
ob_start(); Veriga_Admin::render(); $c = ob_get_clean();
ok( false !== strpos( $c, '<iframe' ) && false !== strpos( $c, 'sandbox="allow-scripts allow-same-origin' ), 'fila Verificator: iframe cu sandbox' );
ok( false !== strpos( $c, 'assets/veriga.html' ), 'iframe către assets/veriga.html' );
$_GET['tab'] = 'scheduled';
ob_start(); Veriga_Admin::render(); $s = ob_get_clean();
ok( false === strpos( $s, '<script>alert' ), 'niciun <script> neescapat în fila Programată' );
ok( false !== strpos( $s, '&lt;script&gt;' ), 'datele ostile apar escapate' );
ok( false !== strpos( $s, '3 broken links' ) && false !== strpos( $s, 'veriga_lc_report&amp;f=html' ), 'rezumat și legături de raport' );
update_option( Veriga_Runner::STATE, array( 'time' => 1, 'ok' => false, 'error' => 'binary_bad_name', 'log' => $evil ) );
ob_start(); Veriga_Admin::render(); $e = ob_get_clean();
ok( false === strpos( $e, '<script>alert' ) && false !== strpos( $e, 'must start with' ), 'stare de eroare: mesaj + log escapat' );

// save(): intrări ostile (matrice, tipuri greșite, valori extreme), fără avertismente PHP
function check_admin_referer( $a ) { return true; }
function wp_unslash( $v ) { return $v; }
function sanitize_key( $k ) { return preg_replace( '/[^a-z0-9_\-]/', '', strtolower( (string) $k ) ); }
function sanitize_email( $e ) { return $e; }
function is_email( $e ) { return (bool) filter_var( $e, FILTER_VALIDATE_EMAIL ); }
function wp_safe_redirect( $u ) { throw new Exception( 'redirect' ); }
function wp_clear_scheduled_hook( $h ) {}
function wp_schedule_event() {}
function wp_die( $m ) { throw new Exception( 'die' ); }
set_error_handler( function ( $no, $str ) { global $fail; if ( ! ( error_reporting() & $no ) ) { return false; } echo "FAIL : avertisment PHP: $str\n"; $fail++; return true; } );
$_POST = array( 'binary' => array( '/x' ), 'frequency' => array( 'daily' ), 'notify' => array(), 'emails' => array( 'a@b.ro' ), 'max_urls' => array( 5 ), 'timeout' => '999999999999', 'no_external' => array( 'x' ) );
try { Veriga_Admin::save(); } catch ( Exception $e ) {}
$st = Veriga_Runner::settings();
ok( 'off' === $st['frequency'] && 'broken' === $st['notify'] && '' === $st['emails'] && 7200 === $st['timeout'] && 5000 === $st['max_urls'], 'save(): intrări de tip matrice neutralizate, valori extreme limitate' );
$_POST = array( 'binary' => '/bin/sh', 'frequency' => 'hourly', 'notify' => 'always', 'emails' => "a@b.ro,\nx@y.zz;nu-e", 'max_urls' => '-9', 'timeout' => '1' );
try { Veriga_Admin::save(); } catch ( Exception $e ) {}
$st = Veriga_Runner::settings();
ok( '' === $st['binary'] && 'off' === $st['frequency'] && 'always' === $st['notify'] && 'a@b.ro, x@y.zz' === $st['emails'] && 100 === $st['max_urls'] && 60 === $st['timeout'], 'save(): binar invalid refuzat, frecvență necunoscută → off, e-mailuri filtrate, limite aplicate' );
restore_error_handler();
echo $fail ? "\n$fail TESTE EȘUATE\n" : "\nTOATE TESTELE ADMIN AU TRECUT\n"; exit( $fail ? 1 : 0 );
