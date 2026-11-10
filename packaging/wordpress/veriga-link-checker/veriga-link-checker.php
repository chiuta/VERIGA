<?php
/**
 * Plugin Name:       VERIGA Link Checker
 * Plugin URI:        https://veriga.chiuta.ro
 * Description:       Link checker (Xenu replica) inside the WordPress admin: the full VERIGA single-file app under Tools → VERIGA, plus an optional scheduled server-side check with the VERIGA binary and e-mail reports. Trade-free: no SaaS, no tracking, no external calls of its own.
 * Version:           1.3.1
 * Requires at least: 6.2
 * Tested up to:      7.1.4
 * Requires PHP:      7.4
 * Author:            Alexio (Alexandru-Ionuț Chiuță)
 * Author URI:        https://trom.tf
 * License:           MIT
 * License URI:       https://opensource.org/licenses/MIT
 * Text Domain:       veriga-link-checker
 * Domain Path:       /languages
 *
 * @package VerigaLinkChecker
 */

defined( 'ABSPATH' ) || exit;

define( 'VERIGA_LC_VERSION', '1.3.1' );
define( 'VERIGA_LC_FILE', __FILE__ );
define( 'VERIGA_LC_DIR', plugin_dir_path( __FILE__ ) );

require_once VERIGA_LC_DIR . 'includes/class-veriga-runner.php';
require_once VERIGA_LC_DIR . 'includes/class-veriga-admin.php';

add_action( 'init', static function () {
	load_plugin_textdomain( 'veriga-link-checker', false, dirname( plugin_basename( VERIGA_LC_FILE ) ) . '/languages' );
} );

add_action( Veriga_Runner::HOOK, array( 'Veriga_Runner', 'cron' ) );
Veriga_Admin::init();

register_activation_hook( __FILE__, array( 'Veriga_Runner', 'reschedule' ) );
register_deactivation_hook( __FILE__, array( 'Veriga_Runner', 'unschedule' ) );
