# Title part
project-create = Proyek baru
project-progress = {$mode ->
        [series] {$name} - #{$episode}
        [movie] {$name}
        [ova] {$name}
        [ova-numbered] {$name} - #{$episode}
        [books] {$name} #{$episode}
        [manga] {$name} #{$episode}
        [light-novel] {$name} #{$episode}
        [games] {$name}
        [vn] {$name}
        *[other] {$name}
    }
project-release = {$name}
project-dropped = Dropped...
project-resumed = Hidup kembali...

# Description part
project-create-desc =
    Proyek {$kind ->
        [series] Serial
        [movie] Film
        [ova] OVA
        [books] Buku
        [manga] Manga
        [light-novel] Light Novel
        [games] Gim
        [vn] Visual Novel
        *[other] Lainnya
    } dengan judul {$name} telah dibuat.

project-dropped-desc =
    Proyek {$name} telah di drop dari grup ini :(

project-resumed-desc =
    Proyek {$name} telah dihidupkan kembali oleh grup ini :)

project-release-desc-header = Rilis!
project-release-desc = {$kind ->
        [single] {$name}
        [episodic] {$name} - {$episodes}
        *[other] {$name}
    } telah dirilis!
project-release-revert-desc-header = Batal rilis...
project-release-revert-desc = Rilisan {$kind ->
        [single] {$name}
        [episodic] {$episodes}
        *[other] {$name}
    } telah dibatalkan dan dikerjakan kembali.

project-episode-range = #{$episode-start} sampai #{$episode-end}
project-episode-single = #{$episode}

project-progress-desc = Status
project-progress-done = ✅ {$role}
project-progress-revert = ❌ {$role}
project-progress-ongoing = ⏳ {$role}
