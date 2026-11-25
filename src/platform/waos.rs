//! Clipboard backend for WaOS, through `wasdk::clipboard`.
//!
//! Plain text (`text/plain`) is carried for now; images, HTML and file lists
//! are left out. Each read and write is checked against the user's clipboard
//! switches for the app, and every call waits on the app's thread until the
//! clipboard answers.

use crate::common::Error;
#[cfg(feature = "image-data")]
use crate::common::ImageData;
use std::{
	borrow::Cow,
	path::{Path, PathBuf},
};
use wasdk::{
	clipboard::{self, ClipboardError, Content, CopyOptions},
	wit_stream,
};

const PLAIN_TEXT: &str = "text/plain";

pub(crate) struct Clipboard {
	clipboard: clipboard::Clipboard,
}

impl Clipboard {
	pub(crate) fn new() -> Result<Clipboard, Error> {
		let clipboard = clipboard::get_clipboard().ok_or(Error::ClipboardNotSupported)?;
		Ok(Clipboard { clipboard })
	}
}

pub(crate) struct Get<'clipboard> {
	clipboard: &'clipboard Clipboard,
}

impl<'clipboard> Get<'clipboard> {
	pub(crate) fn new(clipboard: &'clipboard mut Clipboard) -> Self {
		Self { clipboard }
	}

	pub(crate) fn text(self) -> Result<String, Error> {
		let item = self
			.clipboard
			.clipboard
			.items()
			.into_iter()
			.find(|item| item.types().iter().any(|mime| mime == PLAIN_TEXT))
			.ok_or(Error::ContentNotAvailable)?;
		let bytes = wasdk::block_on(async move {
			let content = item.read(PLAIN_TEXT.to_owned()).await.map_err(read_error)?;
			Ok(content.collect().await)
		})?;
		Ok(String::from_utf8_lossy(&bytes).into_owned())
	}

	pub(crate) fn html(self) -> Result<String, Error> {
		Err(Error::ContentNotAvailable)
	}

	#[cfg(feature = "image-data")]
	pub(crate) fn image(self) -> Result<ImageData<'static>, Error> {
		Err(Error::ContentNotAvailable)
	}

	pub(crate) fn file_list(self) -> Result<Vec<PathBuf>, Error> {
		Err(Error::ContentNotAvailable)
	}
}

pub(crate) struct Set<'clipboard> {
	clipboard: &'clipboard mut Clipboard,
}

impl<'clipboard> Set<'clipboard> {
	pub(crate) fn new(clipboard: &'clipboard mut Clipboard) -> Self {
		Self { clipboard }
	}

	pub(crate) fn text(self, data: Cow<'_, str>) -> Result<(), Error> {
		let bytes = data.into_owned().into_bytes();
		let clipboard = &self.clipboard.clipboard;
		wasdk::block_on(async move {
			let items = vec![vec![(PLAIN_TEXT.to_owned(), Content::Data(stream_of(bytes)))]];
			clipboard.write(items, CopyOptions::new()).await.map_err(write_error)?;
			Ok(())
		})
	}

	pub(crate) fn html(self, _html: Cow<'_, str>, _alt: Option<Cow<'_, str>>) -> Result<(), Error> {
		Err(Error::ConversionFailure)
	}

	#[cfg(feature = "image-data")]
	pub(crate) fn image(self, _data: ImageData) -> Result<(), Error> {
		Err(Error::ConversionFailure)
	}

	pub(crate) fn file_list(self, _file_list: &[impl AsRef<Path>]) -> Result<(), Error> {
		Err(Error::ConversionFailure)
	}
}

pub(crate) struct Clear<'clipboard> {
	clipboard: &'clipboard mut Clipboard,
}

impl<'clipboard> Clear<'clipboard> {
	pub(crate) fn new(clipboard: &'clipboard mut Clipboard) -> Self {
		Self { clipboard }
	}

	pub(crate) fn clear(self) -> Result<(), Error> {
		let clipboard = &self.clipboard.clipboard;
		wasdk::block_on(async move {
			clipboard.write(Vec::new(), CopyOptions::new()).await.map_err(write_error)?;
			Ok(())
		})
	}
}

fn stream_of(bytes: Vec<u8>) -> wasdk::StreamReader<u8> {
	let (mut writer, reader) = wit_stream::new::<u8>();
	wasdk::spawn(async move {
		writer.write_all(bytes).await;
	});
	reader
}

fn read_error(error: ClipboardError) -> Error {
	map_error(error, "the user has turned clipboard reading off for this app")
}

fn write_error(error: ClipboardError) -> Error {
	map_error(error, "the user has turned clipboard writing off for this app")
}

fn map_error(error: ClipboardError, denied_message: &str) -> Error {
	match error {
		ClipboardError::Denied => Error::unknown(denied_message),
		ClipboardError::Changed => Error::ClipboardOccupied,
		ClipboardError::NotFound => Error::ContentNotAvailable,
		ClipboardError::Unsupported => Error::ConversionFailure,
		ClipboardError::Failed(description) => Error::Unknown { description },
	}
}
