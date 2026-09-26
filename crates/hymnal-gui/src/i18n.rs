//! Pushes the `Strings` of a language onto the Slint `I18n` global.

use crate::{AppWindow, I18n};
use hymnal_core::i18n::{Language, Strings};
use slint::ComponentHandle;

/// Fill three `{}` placeholders in `fmt`, in order.
pub fn fill3(fmt: &str, a: &str, b: &str, c: &str) -> String {
    fmt.replacen("{}", a, 1)
        .replacen("{}", b, 1)
        .replacen("{}", c, 1)
}

pub fn apply_language(ui: &AppWindow, lang: Language) -> Strings {
    let s = Strings::for_language(lang);
    let g = ui.global::<I18n>();
    g.set_app_title(s.app_title.clone().into());
    g.set_nav_library(s.nav_library.clone().into());
    g.set_nav_downloader(s.nav_downloader.clone().into());
    g.set_nav_settings(s.nav_settings.clone().into());
    g.set_search_placeholder(s.search_placeholder.clone().into());
    g.set_prev_slide(s.prev_slide.clone().into());
    g.set_next_slide(s.next_slide.clone().into());
    g.set_open_in_powerpoint(s.open_in_powerpoint.clone().into());
    g.set_reveal_in_folder(s.reveal_in_folder.clone().into());
    g.set_downloader_heading(s.downloader_heading.clone().into());
    g.set_downloader_subtitle(s.downloader_subtitle.clone().into());
    g.set_downloader_url_placeholder(s.downloader_url_placeholder.clone().into());
    g.set_choose(s.choose.clone().into());
    g.set_setting_up_downloader(s.setting_up_downloader.clone().into());
    g.set_download_complete(s.download_complete.clone().into());
    g.set_download_failed_prefix(s.download_failed_prefix.clone().into());
    g.set_settings_heading(s.settings_heading.clone().into());
    g.set_version_prefix(s.version_prefix.clone().into());
    g.set_library_heading(s.library_heading.clone().into());
    g.set_force_sync_description(s.force_sync_description.clone().into());
    g.set_force_sync_button(s.force_sync_button.clone().into());
    g.set_syncing_button(s.syncing_button.clone().into());
    g.set_language_heading(s.language_heading.clone().into());
    g.set_user_libraries_heading(s.user_libraries_heading.clone().into());
    g.set_add_folder_button(s.add_folder_button.clone().into());
    g.set_user_libraries_description(s.user_libraries_description.clone().into());
    g.set_library_unavailable_suffix(s.library_unavailable_suffix.clone().into());
    g.set_nav_themes(s.nav_themes.clone().into());
    g.set_themes_heading(s.themes_heading.clone().into());
    g.set_theme_new(s.theme_new.clone().into());
    g.set_theme_save(s.theme_save.clone().into());
    g.set_theme_delete(s.theme_delete.clone().into());
    g.set_theme_preview_sample(s.theme_preview_sample.clone().into());
    g.set_theme_font_label(s.theme_font_label.clone().into());
    g.set_theme_size_label(s.theme_size_label.clone().into());
    g.set_theme_weight_label(s.theme_weight_label.clone().into());
    g.set_theme_align_label(s.theme_align_label.clone().into());
    g.set_theme_text_color_label(s.theme_text_color_label.clone().into());
    g.set_theme_bg_color_label(s.theme_bg_color_label.clone().into());
    g.set_theme_set_active(s.theme_set_active.clone().into());
    g.set_theme_name_label(s.theme_name_label.clone().into());
    g.set_nav_control(s.nav_control.clone().into());
    g.set_control_heading(s.control_heading.clone().into());
    g.set_control_start(s.control_start.clone().into());
    g.set_control_stop(s.control_stop.clone().into());
    g.set_control_blank(s.control_blank.clone().into());
    g.set_control_prev(s.control_prev.clone().into());
    g.set_control_next(s.control_next.clone().into());
    g.set_control_next_label(s.control_next_label.clone().into());
    g.set_control_live(s.control_live.clone().into());
    g.set_control_output(s.control_output.clone().into());
    g.set_control_theme(s.control_theme.clone().into());
    g.set_library_project(s.library_project.clone().into());
    s
}
