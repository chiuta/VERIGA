<?php
/**
 * Server-side runner: executes the VERIGA binary against the site, stores the report and e-mails a summary.
 *
 * The binary is optional. Without it only the in-browser app (Tools → VERIGA) is available.
 *
 * @package VerigaLinkChecker
 */

defined( 'ABSPATH' ) || exit;

final class Veriga_Runner {

	const OPT   = 'veriga_lc_settings';
	const STATE = 'veriga_lc_last';
	const TOKEN = 'veriga_lc_token';
	const LOCK  = 'veriga_lc_lock';
	const HOOK  = 'veriga_lc_run';

	/** Default settings. */
	public static function defaults(): array {
		return array(
			'binary'      => '',
			'frequency'   => 'off',     // off | daily | weekly.
			'notify'      => 'broken',  // never | broken | always.
			'emails'      => '',
			'no_external' => 0,
			'allow_priv'  => 0,
			'max_urls'    => 5000,
			'timeout'     => 900,
		);
	}

	/** Current settings merged over defaults, with the constant override applied. */
	public static function settings(): array {
		$s = get_option( self::OPT, array() );
		$s = array_merge( self::defaults(), is_array( $s ) ? $s : array() );
		if ( defined( 'VERIGA_BINARY' ) && is_string( VERIGA_BINARY ) ) {
			$s['binary'] = VERIGA_BINARY;
		}
		return $s;
	}

	/** Whether the binary path may be edited in the UI (never with DISALLOW_FILE_EDIT, never for non-super-admins on multisite, never when set by constant). */
	public static function can_set_binary(): bool {
		if ( defined( 'VERIGA_BINARY' ) || ( defined( 'DISALLOW_FILE_EDIT' ) && DISALLOW_FILE_EDIT ) ) {
			return false;
		}
		return is_multisite() ? is_super_admin() : current_user_can( 'manage_options' );
	}

	/**
	 * Validates a binary path: absolute, a regular executable file named veriga*, not world-writable.
	 *
	 * @return string Empty string when valid, otherwise a short error code.
	 */
	public static function validate_binary( string $path ): string {
		if ( '' === $path ) {
			return 'empty';
		}
		if ( false !== strpos( $path, "\0" ) || ! preg_match( '#^(/|[A-Za-z]:[\\\\/])#', $path ) ) {
			return 'not_absolute';
		}
		if ( ! preg_match( '/^veriga[A-Za-z0-9._-]*$/', basename( str_replace( '\\', '/', $path ) ) ) ) {
			return 'bad_name';
		}
		if ( ! is_file( $path ) ) {
			return 'not_file';
		}
		if ( ! is_executable( $path ) ) {
			return 'not_executable';
		}
		$perms = @fileperms( $path );
		if ( false !== $perms && ( $perms & 0002 ) ) {
			return 'world_writable';
		}
		return '';
	}

	/** True when proc_open exists and is not disabled. */
	public static function can_exec(): bool {
		if ( ! function_exists( 'proc_open' ) ) {
			return false;
		}
		$disabled = array_map( 'trim', explode( ',', (string) ini_get( 'disable_functions' ) ) );
		return ! in_array( 'proc_open', $disabled, true );
	}

	/** Directory holding the reports: uploads/veriga-lc-<random>/ (created on demand, closed to direct web access). */
	public static function reports_dir( bool $create = false ): string {
		$up    = wp_upload_dir( null, false );
		$token = get_option( self::TOKEN );
		if ( ! is_string( $token ) || ! preg_match( '/^[a-f0-9]{24}$/', $token ) ) {
			$token = bin2hex( random_bytes( 12 ) );
			update_option( self::TOKEN, $token, false );
		}
		$dir = trailingslashit( $up['basedir'] ) . 'veriga-lc-' . $token;
		if ( $create && ! is_dir( $dir ) ) {
			wp_mkdir_p( $dir );
			// Belt and braces: random name + no index + deny rules (Apache 2.4 / 2.2 / IIS).
			file_put_contents( $dir . '/index.php', "<?php\n// Silence is golden.\n" );
			file_put_contents( $dir . '/.htaccess', "<IfModule mod_authz_core.c>\nRequire all denied\n</IfModule>\n<IfModule !mod_authz_core.c>\nDeny from all\n</IfModule>\n" );
			file_put_contents( $dir . '/web.config', '<?xml version="1.0"?><configuration><system.webServer><authorization><deny users="*"/></authorization></system.webServer></configuration>' );
		}
		return $dir;
	}

	/** Deletes the reports directory (used by uninstall). */
	public static function delete_reports_dir(): void {
		$dir = self::reports_dir( false );
		if ( is_dir( $dir ) ) {
			foreach ( (array) scandir( $dir ) as $f ) {
				if ( '.' !== $f && '..' !== $f && is_file( $dir . '/' . $f ) ) {
					@unlink( $dir . '/' . $f );
				}
			}
			@rmdir( $dir );
		}
	}

	/** Command-line arguments (no shell is involved: proc_open receives an array). */
	public static function build_args( array $s, string $home, string $json, string $html, string $lang ): array {
		$a = array(
			$s['binary'],
			$home,
			'--only-broken',
			'-q',
			'--lang',
			'ro' === $lang ? 'ro' : 'en',
			'-t',
			'8',
			'--per-host',
			'4',
			'--max-urls',
			(string) max( 100, min( 1000000, (int) $s['max_urls'] ) ),
			'--exclude',
			'wp-login\\.php|/wp-admin/|[?&]replytocom=|/wp-json/',
			'-o',
			$json,
			'-o',
			$html,
		);
		if ( ! empty( $s['no_external'] ) ) {
			$a[] = '--no-external';
		}
		if ( ! empty( $s['allow_priv'] ) ) {
			$a[] = '--allow-private';
		}
		return $a;
	}

	/** Largest JSON report we parse in memory (bytes). Bigger reports stay downloadable but are not summarized. */
	const MAX_JSON = 33554432;

	/** Strips control characters (incl. CR/LF) and caps the length: values come from the crawled site and are untrusted. */
	public static function clean( $v, int $max = 500 ): string {
		if ( ! is_string( $v ) && ! is_int( $v ) && ! is_float( $v ) ) {
			return '';
		}
		$v = (string) $v;
		$v = preg_replace( '/[\x00-\x1F\x7F\x{2028}\x{2029}]+/u', ' ', $v );
		$v = is_string( $v ) ? $v : '';
		return function_exists( 'mb_substr' ) ? mb_substr( $v, 0, $max ) : substr( $v, 0, $max );
	}

	/** Reads the JSON report and returns [broken count|null, top rows, truncated, cancelled]. A null count means "not summarized". */
	public static function summarize( string $json_file ): array {
		$size = @filesize( $json_file );
		if ( false === $size || $size > self::MAX_JSON ) {
			return array( null, array(), false, false );
		}
		$raw = @file_get_contents( $json_file );
		$d   = is_string( $raw ) ? json_decode( $raw, true, 64 ) : null;
		unset( $raw );
		if ( ! is_array( $d ) || ! isset( $d['results'] ) || ! is_array( $d['results'] ) ) {
			return array( null, array(), false, false );
		}
		// Same set as Status::is_error() in the VERIGA core; 401/403 ("Forbidden") are "to review", not broken.
		$broken = array( 'NotFound', 'ClientError', 'ServerError', 'Timeout', 'NoConnection', 'DnsFailure', 'TlsError', 'TooManyRedirects', 'BadRedirect', 'InvalidUrl', 'InvalidEmail', 'LocalMissing', 'FragmentMissing', 'FtpError' );
		$n      = 0;
		$rows   = array();
		foreach ( $d['results'] as $r ) {
			if ( ! is_array( $r ) || ! isset( $r['status'] ) || ! is_string( $r['status'] ) || ! in_array( $r['status'], $broken, true ) ) {
				continue;
			}
			++$n;
			if ( count( $rows ) < 25 ) {
				$parent = ( isset( $r['parents'] ) && is_array( $r['parents'] ) && isset( $r['parents'][0] ) && is_array( $r['parents'][0] ) ) ? $r['parents'][0]['url'] ?? '' : '';
				$rows[] = array(
					'url'    => self::clean( $r['url'] ?? '', 800 ),
					'status' => self::clean( $r['status'], 40 ),
					'code'   => isset( $r['http_status'] ) && is_numeric( $r['http_status'] ) ? (int) $r['http_status'] : 0,
					'page'   => self::clean( $parent, 800 ),
				);
			}
		}
		return array( $n, $rows, ! empty( $d['truncated'] ), ! empty( $d['cancelled'] ) );
	}

	/** Runs one check now. Returns the stored state array. */
	public static function run(): array {
		$s     = self::settings();
		$state = array(
			'time'  => time(),
			'ok'    => false,
			'error' => '',
		);
		if ( get_transient( self::LOCK ) ) {
			$state['error'] = 'busy';
			return $state;
		}
		if ( ! self::can_exec() ) {
			$state['error'] = 'no_exec';
		} else {
			$bad = self::validate_binary( (string) $s['binary'] );
			if ( '' !== $bad ) {
				$state['error'] = 'binary_' . $bad;
			}
		}
		if ( '' !== $state['error'] ) {
			update_option( self::STATE, $state, false );
			return $state;
		}

		$timeout = max( 60, min( 7200, (int) $s['timeout'] ) );
		set_transient( self::LOCK, time(), $timeout + 60 );
		ignore_user_abort( true );
		if ( function_exists( 'set_time_limit' ) ) {
			@set_time_limit( $timeout + 60 );
		}

		$dir  = self::reports_dir( true );
		$json = $dir . '/report.json';
		$html = $dir . '/report.html';
		@unlink( $json );
		@unlink( $html );
		$lang = str_starts_with( (string) get_locale(), 'ro' ) ? 'ro' : 'en';
		$args = self::build_args( $s, home_url( '/' ), $json, $html, $lang );
		$env  = array(
			'PATH'           => '/usr/local/bin:/usr/bin:/bin',
			'HOME'           => get_temp_dir(),
			'RUST_BACKTRACE' => '0',
		);
		if ( false !== getenv( 'SystemRoot' ) ) {
			$env['SystemRoot'] = (string) getenv( 'SystemRoot' );
		}

		$t0   = microtime( true );
		$proc = @proc_open( $args, array( 0 => array( 'pipe', 'r' ), 1 => array( 'pipe', 'w' ), 2 => array( 'pipe', 'w' ) ), $pipes, $dir, $env );
		$code = -1;
		$out  = '';
		if ( is_resource( $proc ) ) {
			fclose( $pipes[0] );
			stream_set_blocking( $pipes[1], false );
			stream_set_blocking( $pipes[2], false );
			$timed_out = false;
			$open      = array( $pipes[1], $pipes[2] );
			while ( true ) {
				// Wait for data instead of sleeping: a chatty program is drained at full speed, a quiet one costs nothing.
				$r = array_values( array_filter( $open, 'is_resource' ) );
				if ( $r ) {
					$w = null;
					$e = null;
					if ( false !== @stream_select( $r, $w, $e, 0, 200000 ) ) {
						foreach ( $r as $h ) {
							$chunk = fread( $h, 65536 );
							if ( '' !== $chunk && false !== $chunk ) {
								$out .= $chunk;
								if ( strlen( $out ) > 40000 ) {
									$out = substr( $out, -20000 );
								}
							} elseif ( feof( $h ) ) {
								$open = array_values( array_diff( $open, array( $h ) ) );
							}
						}
					}
				} else {
					usleep( 100000 );
				}
				$st = proc_get_status( $proc );
				if ( ! $st['running'] ) {
					$code = (int) $st['exitcode'];
					break;
				}
				if ( microtime( true ) - $t0 > $timeout ) {
					proc_terminate( $proc, 15 );
					usleep( 500000 );
					proc_terminate( $proc, 9 );
					$timed_out = true;
					break;
				}
			}
			// Drain what is left, bounded (a leftover grandchild could keep the pipes open).
			$tail_end = microtime( true ) + 2;
			foreach ( array( $pipes[1], $pipes[2] ) as $h ) {
				while ( microtime( true ) < $tail_end ) {
					$chunk = fread( $h, 65536 );
					if ( '' === $chunk || false === $chunk ) {
						break;
					}
					$out .= $chunk;
					if ( strlen( $out ) > 40000 ) {
						$out = substr( $out, -20000 );
					}
				}
			}
			fclose( $pipes[1] );
			fclose( $pipes[2] );
			$exit = proc_close( $proc );
			if ( -1 === $code && ! $timed_out ) {
				$code = $exit;
			}
			if ( $timed_out ) {
				$state['error'] = 'timeout';
			}
		} else {
			$state['error'] = 'spawn';
		}
		delete_transient( self::LOCK );

		$state['seconds'] = (int) round( microtime( true ) - $t0 );
		$state['exit']    = $code;
		$state['log']     = substr( trim( preg_replace( '/[^\\P{C}\\n\\t]+/u', ' ', $out ) ?? '' ), -2000 );
		// Exit codes: 0 ok, 2 broken links, 3 truncated; anything else is a failure.
		if ( '' === $state['error'] && ! in_array( $code, array( 0, 2, 3 ), true ) ) {
			$state['error'] = 'exit_' . $code;
		}
		if ( '' === $state['error'] ) {
			list( $n, $top, $trunc ) = self::summarize( $json );
			$state['ok']        = true;
			$state['summary']   = null !== $n;
			$state['broken']    = (int) $n;
			$state['top']       = $top;
			$state['truncated'] = $trunc || 3 === $code;
			$state['has_html']  = is_file( $html );
			$state['has_json']  = is_file( $json );
		}
		update_option( self::STATE, $state, false );
		self::notify( $state, $s );
		return $state;
	}

	/** Sends the e-mail summary according to the settings. */
	public static function notify( array $state, array $s ): void {
		if ( 'never' === $s['notify'] ) {
			return;
		}
		$broken = (int) ( $state['broken'] ?? 0 );
		if ( $state['ok'] && 'broken' === $s['notify'] && 0 === $broken && ( ! isset( $state['summary'] ) || $state['summary'] ) ) {
			return;
		}
		$to = array();
		foreach ( preg_split( '/[\s,;]+/', (string) $s['emails'], -1, PREG_SPLIT_NO_EMPTY ) as $e ) {
			if ( is_email( $e ) ) {
				$to[] = $e;
			}
		}
		if ( ! $to ) {
			$to[] = get_option( 'admin_email' );
		}
		$site = self::clean( wp_specialchars_decode( get_bloginfo( 'name' ), ENT_QUOTES ), 120 );
		if ( ! $state['ok'] ) {
			/* translators: 1: site name. */
			$subject = sprintf( __( '[%1$s] VERIGA: the link check failed', 'veriga-link-checker' ), $site );
			$body    = sprintf( "%s\n%s\n", self::error_label( (string) $state['error'] ), (string) ( $state['log'] ?? '' ) );
		} else {
			/* translators: 1: site name, 2: number of broken links. */
			$subject = sprintf( _n( '[%1$s] VERIGA: %2$d broken link', '[%1$s] VERIGA: %2$d broken links', $broken, 'veriga-link-checker' ), $site, $broken );
			$body    = sprintf( "%s\n\n", $subject );
			if ( isset( $state['summary'] ) && ! $state['summary'] ) {
				$body .= __( 'The report is too large to summarize here; open the full report in the admin.', 'veriga-link-checker' ) . "\n";
			}
			foreach ( (array) $state['top'] as $r ) {
				$body .= sprintf( "%s  [%s]\n    %s %s\n", $r['url'], $r['code'] ? $r['code'] : $r['status'], __( 'found on:', 'veriga-link-checker' ), $r['page'] );
			}
			if ( $broken > count( (array) $state['top'] ) ) {
				$body .= "\n" . __( '… and more in the full report.', 'veriga-link-checker' ) . "\n";
			}
		}
		$body .= "\n" . admin_url( 'tools.php?page=veriga-link-checker&tab=scheduled' ) . "\n";
		wp_mail( $to, $subject, $body );
	}

	/** Human-readable label for an error code. */
	public static function error_label( string $code ): string {
		switch ( $code ) {
			case 'busy':
				return __( 'Another check is already running.', 'veriga-link-checker' );
			case 'no_exec':
				return __( 'This server does not allow running programs (proc_open is disabled).', 'veriga-link-checker' );
			case 'binary_empty':
				return __( 'No VERIGA binary is configured.', 'veriga-link-checker' );
			case 'binary_not_absolute':
				return __( 'The binary path must be absolute.', 'veriga-link-checker' );
			case 'binary_bad_name':
				return __( 'The binary file name must start with "veriga".', 'veriga-link-checker' );
			case 'binary_not_file':
				return __( 'The binary file does not exist.', 'veriga-link-checker' );
			case 'binary_not_executable':
				return __( 'The binary file is not executable by the web server user.', 'veriga-link-checker' );
			case 'binary_world_writable':
				return __( 'The binary file is world-writable; refusing to run it.', 'veriga-link-checker' );
			case 'timeout':
				return __( 'The check exceeded the time limit and was stopped.', 'veriga-link-checker' );
			case 'spawn':
				return __( 'The program could not be started.', 'veriga-link-checker' );
		}
		/* translators: %s: error code. */
		return sprintf( __( 'The check failed (%s).', 'veriga-link-checker' ), $code );
	}

	/** WP-Cron callback. */
	public static function cron(): void {
		self::run();
	}

	/** (Re)creates the recurring event according to the settings. */
	public static function reschedule(): void {
		self::unschedule();
		$f = self::settings()['frequency'];
		if ( 'daily' === $f || 'weekly' === $f ) {
			wp_schedule_event( time() + 300, $f === 'daily' ? 'daily' : 'weekly', self::HOOK );
		}
	}

	/** Removes every scheduled event of the plugin. */
	public static function unschedule(): void {
		wp_clear_scheduled_hook( self::HOOK );
	}
}
