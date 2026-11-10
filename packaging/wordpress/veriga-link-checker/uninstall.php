<?php
/**
 * Removes everything the plugin created: options, transients, cron events and stored reports.
 *
 * @package VerigaLinkChecker
 */

defined( 'WP_UNINSTALL_PLUGIN' ) || exit;

require_once __DIR__ . '/includes/class-veriga-runner.php';

Veriga_Runner::unschedule();
Veriga_Runner::delete_reports_dir();
delete_option( Veriga_Runner::OPT );
delete_option( Veriga_Runner::STATE );
delete_option( Veriga_Runner::TOKEN );
delete_transient( Veriga_Runner::LOCK );
