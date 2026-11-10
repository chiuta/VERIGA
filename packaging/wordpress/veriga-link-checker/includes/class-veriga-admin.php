<?php
/**
 * Admin UI: Tools → VERIGA (embedded app + scheduled-check settings) and the report download handler.
 *
 * @package VerigaLinkChecker
 */

defined( 'ABSPATH' ) || exit;

final class Veriga_Admin {

	const PAGE = 'veriga-link-checker';

	public static function init(): void {
		add_action( 'admin_menu', array( __CLASS__, 'menu' ) );
		add_action( 'admin_post_veriga_lc_save', array( __CLASS__, 'save' ) );
		add_action( 'admin_post_veriga_lc_run_now', array( __CLASS__, 'run_now' ) );
		add_action( 'admin_post_veriga_lc_report', array( __CLASS__, 'report' ) );
		add_filter( 'plugin_action_links_' . plugin_basename( VERIGA_LC_FILE ), array( __CLASS__, 'links' ) );
	}

	public static function links( array $l ): array {
		array_unshift( $l, '<a href="' . esc_url( admin_url( 'tools.php?page=' . self::PAGE ) ) . '">' . esc_html__( 'Open', 'veriga-link-checker' ) . '</a>' );
		return $l;
	}

	public static function menu(): void {
		add_management_page( 'VERIGA', 'VERIGA', 'manage_options', self::PAGE, array( __CLASS__, 'render' ) );
	}

	private static function url( string $tab = 'checker', array $extra = array() ): string {
		return add_query_arg( array_merge( array( 'page' => self::PAGE, 'tab' => $tab ), $extra ), admin_url( 'tools.php' ) );
	}

	private static function guard( string $action ): void {
		if ( ! current_user_can( 'manage_options' ) ) {
			wp_die( esc_html__( 'You are not allowed to do this.', 'veriga-link-checker' ), 403 );
		}
		check_admin_referer( $action );
	}

	public static function render(): void {
		if ( ! current_user_can( 'manage_options' ) ) {
			return;
		}
		$tab = isset( $_GET['tab'] ) && 'scheduled' === $_GET['tab'] ? 'scheduled' : 'checker'; // phpcs:ignore WordPress.Security.NonceVerification
		echo '<div class="wrap"><h1>VERIGA <small style="font-weight:400;opacity:.7">' . esc_html( VERIGA_LC_VERSION ) . '</small></h1>';
		echo '<h2 class="nav-tab-wrapper">';
		printf( '<a class="nav-tab%s" href="%s">%s</a>', 'checker' === $tab ? ' nav-tab-active' : '', esc_url( self::url( 'checker' ) ), esc_html__( 'Link checker', 'veriga-link-checker' ) );
		printf( '<a class="nav-tab%s" href="%s">%s</a>', 'scheduled' === $tab ? ' nav-tab-active' : '', esc_url( self::url( 'scheduled' ) ), esc_html__( 'Scheduled check', 'veriga-link-checker' ) );
		echo '</h2>';
		if ( 'checker' === $tab ) {
			self::tab_checker();
		} else {
			self::tab_scheduled();
		}
		echo '</div>';
	}

	private static function tab_checker(): void {
		$src = plugins_url( 'assets/veriga.html', VERIGA_LC_FILE );
		if ( ! is_file( VERIGA_LC_DIR . 'assets/veriga.html' ) ) {
			echo '<div class="notice notice-error"><p>' . esc_html__( 'assets/veriga.html is missing: reinstall the plugin from the release ZIP.', 'veriga-link-checker' ) . '</p></div>';
			return;
		}
		echo '<p>' . esc_html__( 'The checker runs entirely in your browser. Choose “Online site” and enter your site address; it is the same origin as this admin, so no cross-site permission is needed.', 'veriga-link-checker' ) . ' <code>' . esc_html( home_url( '/' ) ) . '</code></p>';
		printf(
			'<iframe title="VERIGA" src="%s" sandbox="allow-scripts allow-same-origin allow-forms allow-downloads allow-modals allow-popups" referrerpolicy="no-referrer" style="width:100%%;height:calc(100vh - 230px);min-height:620px;border:1px solid #c3c4c7;background:#fff"></iframe>',
			esc_url( $src )
		);
	}

	private static function tab_scheduled(): void {
		$s     = Veriga_Runner::settings();
		$state = get_option( Veriga_Runner::STATE );
		$lock  = get_transient( Veriga_Runner::LOCK );
		$next  = wp_next_scheduled( Veriga_Runner::HOOK );
		$can_b = Veriga_Runner::can_set_binary();

		if ( isset( $_GET['saved'] ) ) { // phpcs:ignore WordPress.Security.NonceVerification
			echo '<div class="notice notice-success is-dismissible"><p>' . esc_html__( 'Settings saved.', 'veriga-link-checker' ) . '</p></div>';
		}
		if ( isset( $_GET['started'] ) ) { // phpcs:ignore WordPress.Security.NonceVerification
			echo '<div class="notice notice-info is-dismissible"><p>' . esc_html__( 'The check was queued and will start in the background shortly. Reload this page to see the result.', 'veriga-link-checker' ) . '</p></div>';
		}
		if ( ! Veriga_Runner::can_exec() ) {
			echo '<div class="notice notice-warning"><p>' . esc_html( Veriga_Runner::error_label( 'no_exec' ) ) . ' ' . esc_html__( 'Use the Link checker tab instead.', 'veriga-link-checker' ) . '</p></div>';
		}

		echo '<p>' . esc_html__( 'Optional: a server-side check with the VERIGA command-line program (a static binary from the GitHub releases, e.g. the “linux-musl” build). It crawls the public site address, stores an HTML/JSON report and e-mails a summary.', 'veriga-link-checker' ) . '</p>';

		echo '<form method="post" action="' . esc_url( admin_url( 'admin-post.php' ) ) . '"><input type="hidden" name="action" value="veriga_lc_save">';
		wp_nonce_field( 'veriga_lc_save' );
		echo '<table class="form-table" role="presentation">';

		echo '<tr><th><label for="vlc-bin">' . esc_html__( 'VERIGA binary (absolute path)', 'veriga-link-checker' ) . '</label></th><td>';
		printf( '<input id="vlc-bin" name="binary" type="text" class="regular-text code" value="%s" placeholder="/usr/local/bin/veriga"%s>', esc_attr( $s['binary'] ), $can_b ? '' : ' disabled' );
		if ( ! $can_b ) {
			echo '<p class="description">' . esc_html__( 'Locked: set by the VERIGA_BINARY constant, or editing is disabled on this installation (DISALLOW_FILE_EDIT / multisite).', 'veriga-link-checker' ) . '</p>';
		} else {
			echo '<p class="description">' . esc_html__( 'The file name must start with “veriga”. Keep the file outside the web root and not writable by the web server.', 'veriga-link-checker' ) . '</p>';
		}
		echo '</td></tr>';

		echo '<tr><th><label for="vlc-freq">' . esc_html__( 'Frequency', 'veriga-link-checker' ) . '</label></th><td><select id="vlc-freq" name="frequency">';
		foreach ( array( 'off' => __( 'Off (manual only)', 'veriga-link-checker' ), 'daily' => __( 'Daily', 'veriga-link-checker' ), 'weekly' => __( 'Weekly', 'veriga-link-checker' ) ) as $k => $l ) {
			printf( '<option value="%s"%s>%s</option>', esc_attr( $k ), selected( $s['frequency'], $k, false ), esc_html( $l ) );
		}
		echo '</select></td></tr>';

		echo '<tr><th><label for="vlc-notify">' . esc_html__( 'E-mail', 'veriga-link-checker' ) . '</label></th><td><select id="vlc-notify" name="notify">';
		foreach ( array( 'broken' => __( 'Only when links are broken (or the check fails)', 'veriga-link-checker' ), 'always' => __( 'After every check', 'veriga-link-checker' ), 'never' => __( 'Never', 'veriga-link-checker' ) ) as $k => $l ) {
			printf( '<option value="%s"%s>%s</option>', esc_attr( $k ), selected( $s['notify'], $k, false ), esc_html( $l ) );
		}
		printf( '</select><p><input name="emails" type="text" class="regular-text" value="%s" placeholder="%s"></p><p class="description">%s</p></td></tr>', esc_attr( $s['emails'] ), esc_attr( get_option( 'admin_email' ) ), esc_html__( 'Recipients, separated by commas. Empty = the site administrator address.', 'veriga-link-checker' ) );

		echo '<tr><th>' . esc_html__( 'Options', 'veriga-link-checker' ) . '</th><td>';
		printf( '<label><input type="checkbox" name="no_external" value="1"%s> %s</label><br>', checked( ! empty( $s['no_external'] ), true, false ), esc_html__( 'Skip external links (check only this site)', 'veriga-link-checker' ) );
		printf( '<label><input type="checkbox" name="allow_priv" value="1"%s> %s</label>', checked( ! empty( $s['allow_priv'] ), true, false ), esc_html__( 'Allow private/local addresses (needed when the site address resolves to a private IP, e.g. intranet or Docker)', 'veriga-link-checker' ) );
		echo '</td></tr>';

		printf( '<tr><th><label for="vlc-max">%s</label></th><td><input id="vlc-max" name="max_urls" type="number" min="100" max="1000000" value="%d"> &nbsp; <label for="vlc-to">%s</label> <input id="vlc-to" name="timeout" type="number" min="60" max="7200" value="%d"></td></tr>', esc_html__( 'Maximum URLs', 'veriga-link-checker' ), (int) $s['max_urls'], esc_html__( 'Time limit (seconds)', 'veriga-link-checker' ), (int) $s['timeout'] );
		echo '</table>';
		submit_button( __( 'Save settings', 'veriga-link-checker' ) );
		echo '</form>';

		echo '<hr><h2>' . esc_html__( 'Last check', 'veriga-link-checker' ) . '</h2>';
		if ( $lock ) {
			echo '<p><em>' . esc_html__( 'A check is running now…', 'veriga-link-checker' ) . '</em></p>';
		}
		if ( $next ) {
			/* translators: %s: date and time. */
			echo '<p>' . esc_html( sprintf( __( 'Next scheduled check: %s', 'veriga-link-checker' ), wp_date( 'Y-m-d H:i', $next ) ) ) . '</p>';
		}
		if ( is_array( $state ) ) {
			/* translators: %s: date and time. */
			echo '<p>' . esc_html( sprintf( __( 'Finished: %s', 'veriga-link-checker' ), wp_date( 'Y-m-d H:i', (int) $state['time'] ) ) ) . '</p>';
			if ( empty( $state['ok'] ) ) {
				echo '<div class="notice notice-error inline"><p>' . esc_html( Veriga_Runner::error_label( (string) $state['error'] ) ) . '</p>';
				if ( ! empty( $state['log'] ) ) {
					echo '<pre style="white-space:pre-wrap">' . esc_html( $state['log'] ) . '</pre>';
				}
				echo '</div>';
			} else {
				$n = (int) $state['broken'];
				/* translators: %d: number of broken links. */
				echo '<p><strong>' . esc_html( sprintf( _n( '%d broken link', '%d broken links', $n, 'veriga-link-checker' ), $n ) ) . '</strong>';
				if ( ! empty( $state['truncated'] ) ) {
					echo ' — ' . esc_html__( 'the crawl was truncated (URL limit reached); raise the limit for a full result.', 'veriga-link-checker' );
				}
				echo '</p>';
				if ( ! empty( $state['top'] ) ) {
					echo '<table class="widefat striped" style="max-width:1100px"><thead><tr><th>URL</th><th>' . esc_html__( 'Status', 'veriga-link-checker' ) . '</th><th>' . esc_html__( 'Found on', 'veriga-link-checker' ) . '</th></tr></thead><tbody>';
					foreach ( $state['top'] as $r ) {
						printf( '<tr><td style="word-break:break-all">%s</td><td>%s</td><td style="word-break:break-all">%s</td></tr>', esc_html( $r['url'] ), esc_html( $r['code'] ? $r['code'] : $r['status'] ), esc_html( $r['page'] ) );
					}
					echo '</tbody></table>';
				}
				foreach ( array( 'html' => 'HTML', 'json' => 'JSON' ) as $f => $label ) {
					if ( ! empty( $state[ 'has_' . $f ] ) ) {
						printf( '<a class="button" href="%s" target="_blank" rel="noopener">%s</a> ', esc_url( wp_nonce_url( admin_url( 'admin-post.php?action=veriga_lc_report&f=' . $f ), 'veriga_lc_report' ) ), esc_html( sprintf( /* translators: %s: format. */ __( 'Report %s', 'veriga-link-checker' ), $label ) ) );
					}
				}
			}
		} else {
			echo '<p>' . esc_html__( 'No check has run yet.', 'veriga-link-checker' ) . '</p>';
		}

		echo '<form method="post" action="' . esc_url( admin_url( 'admin-post.php' ) ) . '" style="margin-top:14px"><input type="hidden" name="action" value="veriga_lc_run_now">';
		wp_nonce_field( 'veriga_lc_run_now' );
		submit_button( __( 'Run a check now', 'veriga-link-checker' ), 'secondary', 'submit', false, $lock ? array( 'disabled' => 'disabled' ) : array() );
		echo '</form>';
	}

	public static function save(): void {
		self::guard( 'veriga_lc_save' );
		$old = Veriga_Runner::settings();
		$new = $old;
		// phpcs:disable WordPress.Security.NonceVerification -- verified in guard().
		if ( Veriga_Runner::can_set_binary() && isset( $_POST['binary'] ) ) {
			$b = trim( wp_unslash( (string) $_POST['binary'] ) );
			if ( '' === $b || '' === Veriga_Runner::validate_binary( $b ) ) {
				$new['binary'] = $b;
			}
		}
		$f                  = isset( $_POST['frequency'] ) ? sanitize_key( wp_unslash( $_POST['frequency'] ) ) : 'off';
		$new['frequency']   = in_array( $f, array( 'off', 'daily', 'weekly' ), true ) ? $f : 'off';
		$n                  = isset( $_POST['notify'] ) ? sanitize_key( wp_unslash( $_POST['notify'] ) ) : 'broken';
		$new['notify']      = in_array( $n, array( 'never', 'broken', 'always' ), true ) ? $n : 'broken';
		$mails              = array_filter( preg_split( '/[\s,;]+/', isset( $_POST['emails'] ) ? wp_unslash( (string) $_POST['emails'] ) : '', -1, PREG_SPLIT_NO_EMPTY ), 'is_email' );
		$new['emails']      = implode( ', ', array_map( 'sanitize_email', $mails ) );
		$new['no_external'] = empty( $_POST['no_external'] ) ? 0 : 1;
		$new['allow_priv']  = empty( $_POST['allow_priv'] ) ? 0 : 1;
		$new['max_urls']    = max( 100, min( 1000000, (int) ( $_POST['max_urls'] ?? 5000 ) ) );
		$new['timeout']     = max( 60, min( 7200, (int) ( $_POST['timeout'] ?? 900 ) ) );
		// phpcs:enable
		update_option( Veriga_Runner::OPT, $new, false );
		Veriga_Runner::reschedule();
		wp_safe_redirect( self::url( 'scheduled', array( 'saved' => 1 ) ) );
		exit;
	}

	public static function run_now(): void {
		self::guard( 'veriga_lc_run_now' );
		if ( ! get_transient( Veriga_Runner::LOCK ) ) {
			wp_schedule_single_event( time() + 1, Veriga_Runner::HOOK );
			spawn_cron();
		}
		wp_safe_redirect( self::url( 'scheduled', array( 'started' => 1 ) ) );
		exit;
	}

	/** Streams a stored report. HTML is served sandboxed with a locked-down CSP. */
	public static function report(): void {
		if ( ! current_user_can( 'manage_options' ) ) {
			wp_die( esc_html__( 'You are not allowed to do this.', 'veriga-link-checker' ), 403 );
		}
		check_admin_referer( 'veriga_lc_report' );
		$f    = isset( $_GET['f'] ) && 'json' === $_GET['f'] ? 'json' : 'html';
		$file = Veriga_Runner::reports_dir( false ) . '/report.' . $f;
		if ( ! is_file( $file ) ) {
			wp_die( esc_html__( 'The report does not exist.', 'veriga-link-checker' ), 404 );
		}
		nocache_headers();
		header( 'X-Content-Type-Options: nosniff' );
		header( 'Referrer-Policy: no-referrer' );
		if ( 'html' === $f ) {
			header( 'Content-Type: text/html; charset=utf-8' );
			header( "Content-Security-Policy: sandbox; default-src 'none'; style-src 'unsafe-inline'; img-src data:" );
		} else {
			header( 'Content-Type: application/json; charset=utf-8' );
			header( 'Content-Disposition: attachment; filename="veriga-report.json"' );
		}
		header( 'Content-Length: ' . filesize( $file ) );
		readfile( $file ); // phpcs:ignore WordPress.WP.AlternativeFunctions
		exit;
	}
}
