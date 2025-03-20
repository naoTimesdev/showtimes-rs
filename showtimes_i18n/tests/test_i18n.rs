#[test]
fn test_i18n_role() {
    assert_eq!(
        showtimes_i18n::t("role-pr", Some(showtimes_i18n::Language::Id)),
        "Suntingan"
    );

    assert_eq!(
        showtimes_i18n::t("role-actor-pr", Some(showtimes_i18n::Language::Id)),
        "Penyunting"
    );
}

#[test]
fn test_i18n_webhook_create() {
    assert_eq!(
        showtimes_i18n::t("project-create", Some(showtimes_i18n::Language::Id)),
        "Proyek baru"
    );

    assert_eq!(
        showtimes_i18n::tr(
            "project-create-desc",
            Some(showtimes_i18n::Language::Id),
            &[
                ("kind", "movie".to_string()),
                ("name", "Film baru".to_string())
            ],
        ),
        "Proyek Film dengan judul Film baru telah dibuat."
    );

    assert_eq!(
        showtimes_i18n::tr(
            "project-create-desc",
            Some(showtimes_i18n::Language::Id),
            &[
                ("kind", "unknown".to_string()),
                ("name", "Film baru".to_string())
            ],
        ),
        "Proyek Lainnya dengan judul Film baru telah dibuat."
    );
}

#[test]
fn test_i18n_webhook_dropped() {
    assert_eq!(
        showtimes_i18n::t("project-dropped", Some(showtimes_i18n::Language::Id)),
        "Dropped..."
    );

    assert_eq!(
        showtimes_i18n::tr(
            "project-dropped-desc",
            Some(showtimes_i18n::Language::Id),
            &[("name", "Film baru".to_string())],
        ),
        "Proyek Film baru telah di drop dari grup ini :("
    );
}

#[test]
fn test_i18n_webhook_resumed() {
    assert_eq!(
        showtimes_i18n::t("project-resumed", Some(showtimes_i18n::Language::Id)),
        "Hidup kembali..."
    );

    assert_eq!(
        showtimes_i18n::tr(
            "project-resumed-desc",
            Some(showtimes_i18n::Language::Id),
            &[("name", "Film baru".to_string())],
        ),
        "Proyek Film baru telah dihidupkan kembali oleh grup ini :)"
    );
}

#[test]
fn test_i18n_webhook_released() {
    assert_eq!(
        showtimes_i18n::tr(
            "project-release",
            Some(showtimes_i18n::Language::Id),
            &[("name", "Anime baru".to_string())]
        ),
        "Anime baru"
    );

    assert_eq!(
        showtimes_i18n::t(
            "project-release-desc-header",
            Some(showtimes_i18n::Language::Id)
        ),
        "Rilis!"
    );

    assert_eq!(
        showtimes_i18n::tr(
            "project-release-desc",
            Some(showtimes_i18n::Language::Id),
            &[
                ("kind", "single".to_string()),
                ("name", "Film baru".to_string()),
            ],
        ),
        "Film baru telah dirilis!"
    );

    let episode_single = showtimes_i18n::tr(
        "project-episode-single",
        Some(showtimes_i18n::Language::Id),
        &[("episode", "10".to_string())],
    );
    let episode_range = showtimes_i18n::tr(
        "project-episode-range",
        Some(showtimes_i18n::Language::Id),
        &[
            ("episode-start", "1".to_string()),
            ("episode-end", "2".to_string()),
        ],
    );

    assert_eq!(episode_single, "#10");
    assert_eq!(episode_range, "#1 sampai #2");

    assert_eq!(
        showtimes_i18n::tr(
            "project-release-desc",
            Some(showtimes_i18n::Language::Id),
            &[
                ("kind", "episodic".to_string()),
                ("name", "Anime baru".to_string()),
                ("episodes", episode_single),
            ],
        ),
        "Anime baru - #10 telah dirilis!"
    );
    assert_eq!(
        showtimes_i18n::tr(
            "project-release-desc",
            Some(showtimes_i18n::Language::Id),
            &[
                ("kind", "episodic".to_string()),
                ("name", "Anime baru".to_string()),
                ("episodes", episode_range),
            ],
        ),
        "Anime baru - #1 sampai #2 telah dirilis!"
    );
}

#[test]
fn test_i18n_webhook_unreleased() {
    assert_eq!(
        showtimes_i18n::tr(
            "project-release",
            Some(showtimes_i18n::Language::Id),
            &[("name", "Anime baru".to_string())]
        ),
        "Anime baru"
    );

    assert_eq!(
        showtimes_i18n::t(
            "project-release-revert-desc-header",
            Some(showtimes_i18n::Language::Id)
        ),
        "Batal rilis..."
    );

    assert_eq!(
        showtimes_i18n::tr(
            "project-release-revert-desc",
            Some(showtimes_i18n::Language::Id),
            &[
                ("kind", "single".to_string()),
                ("name", "Film baru".to_string()),
            ],
        ),
        "Rilisan Film baru telah dibatalkan dan dikerjakan kembali."
    );

    let episode_single = showtimes_i18n::tr(
        "project-episode-single",
        Some(showtimes_i18n::Language::Id),
        &[("episode", "12".to_string())],
    );
    let episode_range = showtimes_i18n::tr(
        "project-episode-range",
        Some(showtimes_i18n::Language::Id),
        &[
            ("episode-start", "4".to_string()),
            ("episode-end", "6".to_string()),
        ],
    );

    assert_eq!(episode_single, "#12");
    assert_eq!(episode_range, "#4 sampai #6");

    assert_eq!(
        showtimes_i18n::tr(
            "project-release-revert-desc",
            Some(showtimes_i18n::Language::Id),
            &[
                ("kind", "episodic".to_string()),
                ("name", "Anime baru".to_string()),
                ("episodes", episode_single),
            ],
        ),
        "Rilisan #12 telah dibatalkan dan dikerjakan kembali."
    );
    assert_eq!(
        showtimes_i18n::tr(
            "project-release-revert-desc",
            Some(showtimes_i18n::Language::Id),
            &[
                ("kind", "episodic".to_string()),
                ("name", "Anime baru".to_string()),
                ("episodes", episode_range),
            ],
        ),
        "Rilisan #4 sampai #6 telah dibatalkan dan dikerjakan kembali."
    );
}

#[test]
fn test_i18n_webhook_progress() {
    assert_eq!(
        showtimes_i18n::tr(
            "project-progress",
            Some(showtimes_i18n::Language::Id),
            &[
                ("name", "Anime baru".to_string()),
                ("mode", "series".to_string()),
                ("episode", "12".to_string()),
            ]
        ),
        "Anime baru - #12"
    );

    assert_eq!(
        showtimes_i18n::tr(
            "project-progress",
            Some(showtimes_i18n::Language::Id),
            &[
                ("name", "Film baru".to_string()),
                ("mode", "movie".to_string()),
            ]
        ),
        "Film baru"
    );

    assert_eq!(
        showtimes_i18n::tr(
            "project-progress",
            Some(showtimes_i18n::Language::Id),
            &[
                ("name", "Novel".to_string()),
                ("mode", "light-novel".to_string()),
                ("episode", "12".to_string()),
            ]
        ),
        "Novel #12"
    );

    let role = showtimes_i18n::t("role-tl", Some(showtimes_i18n::Language::Id));

    assert_eq!(
        showtimes_i18n::tr(
            "project-progress-done",
            Some(showtimes_i18n::Language::Id),
            &[("role", role.to_string()),]
        ),
        "✅ Terjemahan"
    );
    assert_eq!(
        showtimes_i18n::tr(
            "project-progress-revert",
            Some(showtimes_i18n::Language::Id),
            &[("role", role.to_string()),]
        ),
        "❌ Terjemahan"
    );
    assert_eq!(
        showtimes_i18n::tr(
            "project-progress-ongoing",
            Some(showtimes_i18n::Language::Id),
            &[("role", role.to_string()),]
        ),
        "⏳ Terjemahan"
    );
}
