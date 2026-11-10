<?php
/** Teste adversariale: binar ostil (JSON/ieșire/timp), valori necontrolate din site, injecție în e-mail. Folosește aceleași stub-uri ca run.php. */
define( 'ABSPATH', __DIR__ );
$GLOBALS['opts'] = $GLOBALS['trans'] = $GLOBALS['mails'] = array();
$tmp = sys_get_temp_dir() . '/vlc-host-' . getmypid(); @mkdir( $tmp . '/uploads', 0777, true ); $GLOBALS['tmp'] = $tmp;
function get_option( $k, $d = false ) { return $GLOBALS['opts'][ $k ] ?? $d; }
function update_option( $k, $v, $a = null ) { $GLOBALS['opts'][ $k ] = $v; return true; }
function get_transient( $k ) { return $GLOBALS['trans'][ $k ] ?? false; }
function set_transient( $k, $v, $e = 0 ) { $GLOBALS['trans'][ $k ] = $v; return true; }
function delete_transient( $k ) { unset( $GLOBALS['trans'][ $k ] ); }
function wp_upload_dir( $t = null, $c = true ) { return array( 'basedir' => $GLOBALS['tmp'] . '/uploads' ); }
function trailingslashit( $s ) { return rtrim( $s, '/\\' ) . '/'; }
function wp_mkdir_p( $d ) { return @mkdir( $d, 0777, true ); }
function home_url( $p = '' ) { return 'http://127.0.0.1:8777/'; }
function get_locale() { return 'en_US'; }
function get_temp_dir() { return sys_get_temp_dir() . '/'; }
function is_email( $e ) { return (bool) filter_var( $e, FILTER_VALIDATE_EMAIL ); }
function get_bloginfo( $k ) { return $GLOBALS['sitename']; }
function wp_specialchars_decode( $s, $q = 0 ) { return $s; }
function admin_url( $p = '' ) { return 'https://x.test/wp-admin/' . $p; }
function __( $s, $d = '' ) { return $s; }
function _n( $a, $b, $n, $d = '' ) { return 1 === $n ? $a : $b; }
function is_multisite() { return false; }
function wp_mail( $to, $s, $b ) { $GLOBALS['mails'][] = compact( 'to', 's', 'b' ); return true; }
$GLOBALS['sitename'] = "Site\r\nBcc: victima@evil.test\r\nSubject: fals";
require __DIR__ . '/../veriga-link-checker/includes/class-veriga-runner.php';
$fail = 0;
function ok( $c, $m ) { global $fail; echo ( $c ? 'ok   : ' : 'FAIL : ' ) . $m . "\n"; if ( ! $c ) { $fail++; } }
function fake( $name, $script ) { global $tmp; $p = "$tmp/$name"; file_put_contents( $p, "#!/bin/sh\n" . $script ); chmod( $p, 0755 ); update_option( Veriga_Runner::OPT, array_merge( Veriga_Runner::defaults(), array( 'binary' => $p ) ) ); $GLOBALS['mails'] = array(); return $p; }
// scriu JSON ostil în fișierul dat după „-o” (primul): găsim calea în argumentele scriptului
$wr = 'while [ $# -gt 0 ]; do if [ "$1" = "-o" ]; then out="$2"; break; fi; shift; done; ';

// 1) tipuri ostile în JSON: nu trebuie să producă avertismente/erori fatale; valorile sunt curățate
$json = json_encode( array( 'results' => array(
	array( 'url' => "http://a/\r\nBcc: x@evil.test\x00", 'status' => 'NotFound', 'http_status' => '404', 'parents' => 'sir' ),
	array( 'url' => array( 'x' ), 'status' => array( 'NotFound' ) ),
	array( 'url' => 'http://b/', 'status' => 'ServerError', 'http_status' => array( 1 ), 'parents' => array( 'nu-obiect' ) ),
	array( 'url' => 'http://c/', 'status' => 'Ok' ),
	'nu-array', null, 7,
	array( 'url' => 'http://d/<script>alert(1)</script>', 'status' => 'Forbidden' ),
	array( 'url' => 'http://e/', 'status' => 'Timeout', 'parents' => array( array( 'url' => "http://p/\n\nTo: x" ) ) ),
), 'truncated' => 'da' ) );
file_put_contents( "$tmp/hostile1.json", $json );
fake( 'veriga-h1', $wr . "cp $tmp/hostile1.json \"\$out\"; exit 2\n" );
set_error_handler( function ( $no, $str ) { global $fail; if ( ! ( error_reporting() & $no ) ) { return false; } echo "FAIL : avertisment PHP: $str\n"; $fail++; return true; } );
$st = Veriga_Runner::run();
restore_error_handler();
ok( $st['ok'] && 3 === $st['broken'], 'JSON ostil: 3 rupte (NotFound, ServerError, Timeout), restul ignorat fără avertismente' );
ok( 0 === preg_match( '/[\x00-\x1f]/', $st['top'][0]['url'] . $st['top'][2]['page'] ), 'URL/pagină curățate de caractere de control' );
$m = $GLOBALS['mails'][0] ?? array( 's' => '', 'b' => '' );
ok( 1 === count( $GLOBALS['mails'] ) && false === strpbrk( $m['s'], "\r\n" ), 'subiectul e-mailului fără CR/LF (nume site ostil)' );
ok( false === strpos( $m['s'], "\nBcc" ) && 1 === preg_match( '/^\[Site Bcc: victima@evil\.test Subject: fals\] VERIGA: 3 broken links$/', $m['s'] ), 'nume de site cu CRLF neutralizat în subiect' );
ok( 0 === preg_match( '/^(Bcc|To):/mi', $m['b'] ), 'corp e-mail: nicio linie de antet injectată din URL-uri' );

// 2) raport enorm: nu se încarcă în memorie
$fh = fopen( "$tmp/big.json", 'w' ); fwrite( $fh, '{"results":[' ); $chunk = str_repeat( '{"url":"http://a/","status":"NotFound"},', 20000 ); for ( $i = 0; $i < 60; $i++ ) { fwrite( $fh, $chunk ); } fwrite( $fh, '{"url":"http://z/","status":"NotFound"}]}' ); fclose( $fh );
ok( filesize( "$tmp/big.json" ) > Veriga_Runner::MAX_JSON, 'raportul de test depășește limita (' . round( filesize( "$tmp/big.json" ) / 1048576 ) . ' MB)' );
fake( 'veriga-h2', $wr . "cp $tmp/big.json \"\$out\"; exit 2\n" );
$m0 = memory_get_peak_usage( true ); $st = Veriga_Runner::run();
ok( $st['ok'] && false === $st['summary'] && ( memory_get_peak_usage( true ) - $m0 ) < 20 * 1048576, 'raport enorm: nu e încărcat (memorie extra < 20 MB), starea „nerezumat”' );
ok( 1 === count( $GLOBALS['mails'] ) && false !== strpos( $GLOBALS['mails'][0]['b'], 'too large' ), 'e-mail trimis chiar dacă numărătoarea e necunoscută (notify=broken)' );

// 3) JSON invalid / adânc / gol
foreach ( array( 'invalid' => '{{{', 'adanc' => str_repeat( '[', 5000 ), 'gol' => '' ) as $k => $body ) {
	file_put_contents( "$tmp/j_$k.json", $body ); fake( "veriga-j$k", $wr . "cp $tmp/j_$k.json \"\$out\"; exit 2\n" );
	$st = Veriga_Runner::run(); ok( $st['ok'] && false === $st['summary'], "JSON $k: tratat fără eroare, nerezumat" );
}

// 4) ieșire uriașă pe stdout/stderr: memorie și jurnal limitate
fake( 'veriga-h3', "head -c 300000000 /dev/zero | tr '\\0' 'A'; head -c 100000000 /dev/zero | tr '\\0' 'B' >&2; exit 0\n" );
$t = microtime( true ); $st = Veriga_Runner::run();
ok( strlen( $st['log'] ) <= 2000 && ( microtime( true ) - $t ) < 60, 'ieșire de 400 MB: jurnal ≤ 2000 octeți, fără blocaj (' . round( microtime( true ) - $t ) . ' s, vârf memorie ' . round( memory_get_peak_usage( true ) / 1048576 ) . ' MB)' );
ok( memory_get_peak_usage( true ) < 128 * 1048576, 'memorie PHP sub 128 MB' );

// 5) caractere de control în jurnal
fake( 'veriga-h4', "printf 'x\\033[31m\\033]0;titlu\\007y\\r\\n' >&2; exit 1\n" );
$st = Veriga_Runner::run();
ok( ! $st['ok'] && 0 === preg_match( '/[\x00-\x08\x0b-\x1f\x7f]/', $st['log'] ), 'jurnal fără secvențe de control/escape' );

// 6) binar care nu se termină: oprit la timeout (min. 60 s), apoi blocarea eliberată
if ( getenv( 'VLC_SLOW' ) ) {
	fake( 'veriga-h5', "sleep 600\n" );
	update_option( Veriga_Runner::OPT, array_merge( get_option( Veriga_Runner::OPT ), array( 'timeout' => 60 ) ) );
	$t = microtime( true ); $st = Veriga_Runner::run(); $dt = microtime( true ) - $t;
	ok( 'timeout' === $st['error'] && $dt < 75 && ! get_transient( Veriga_Runner::LOCK ), 'binar blocat: oprit la timeout (' . round( $dt ) . ' s), blocare eliberată' );
}
echo $fail ? "\n$fail TESTE EȘUATE\n" : "\nTOATE TESTELE OSTILE AU TRECUT\n";
exit( $fail ? 1 : 0 );
