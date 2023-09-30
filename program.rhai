const MIME_TYPES = #{
    image: [
        "image/jpeg",
        "image/png",
        "image/gif",
        "image/svg" 
    ],
    video: [
        "video/mp4",
        "video/webm",
        "video/ogg"
    ]
};

/// Returns true if the given `mime_type` is a video type.
fn is_video(mime_type) {
    return global::MIME_TYPES.video.contains(mime_type);
}

/// Returns true if the given `mime_type` is an image type.
fn is_image(mime_type) {
    return global::MIME_TYPES.image.contains(mime_type);
}

// Use a prefix for all object keys.
let prefix = "~mk/";

// Guess the mime type of the file based on the name.
let mime_type = mime::guess_from_path(path);
object.content_type = mime_type;

// Set the content-disposition to inline if it's a well-supported type.
object.content_disposition = if is_video(mime_type) || is_image(mime_type) {
    "inline"
} else {
    "attachment"
};

// Put videos in `v/<name>` and images in `i/<name>`, otherwise just use the
// plain file name.
object.key = prefix + if is_video(mime_type) {
    `v/${path.file_name}`
} else if is_image(mime_type) {
    `i/${path.file_name}`
} else {
    `${path.file_name}`
};
