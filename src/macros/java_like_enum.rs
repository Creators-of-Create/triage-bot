#[macro_export]
macro_rules! java_like_enum {
	($visibility: vis enum $name: ident ( $($attribname: ident : $attribtype: ty),* ) { $($entryname: ident ( $($fieldvalue: expr),* ) ; )* } ) => {
		java_like_enum!(
			@recurse1
			$visibility enum $name ;
			$($entryname),* ;
			$($attribname : $attribtype),* ;
			$($entryname ; $($fieldvalue,)* ;)* ; !
		);
	};

	(@recurse1 $visibility: vis enum $name: ident ; $($entryname1: ident),* ; $($attribname: ident : $attribtype: ty),* ; $($ignored: ident ; ;)* ; $($(($zipped_entryname: ident, $zipped_fieldvalue: expr),)* ;)* ) => {
		java_like_enum!(
			@recurse2
			$visibility enum $name ;
			$($entryname1),* ;
			$($attribname : $attribtype),* ;
			$($(($zipped_entryname, $zipped_fieldvalue),)* ;)* !
		);
	};

	(@recurse1 $visibility: vis enum $name: ident ; $($entryname1: ident),* ; $($attribname: ident : $attribtype: ty),* ; $($entryname2: ident ; $fieldvalue: expr, $($restfields:expr,)* ;)* ; ! ) => {
		java_like_enum!(
			@recurse1
			$visibility enum $name ;
			$($entryname1),* ;
			$($attribname : $attribtype),* ;
			$($entryname2 ; $($restfields,)* ;)* ;
			$(($entryname2, $fieldvalue), ;)*
		);
	};

	(@recurse1 $visibility: vis enum $name: ident ; $($entryname1: ident),* ; $($attribname: ident : $attribtype: ty),* ; $($entryname2: ident ; $fieldvalue: expr, $($restfields:expr,)* ;)* ; $($(($zipped_entryname: ident, $zipped_fieldvalue: expr),)* ;)* ) => {
		java_like_enum!(
			@recurse1
			$visibility enum $name ;
			$($entryname1),* ;
			$($attribname : $attribtype),* ;
			$($entryname2 ; $($restfields,)* ;)* ;
			$($(($zipped_entryname, $zipped_fieldvalue),)* ($entryname2, $fieldvalue), ;)*
		);
	};

	(@recurse2 $visibility: vis enum $name: ident ; $($entryname1: ident),* ; $($attribname: ident : $attribtype: ty),* ; $(;)* ! $($(($entryname2: ident, $fieldvalue: expr),)* ;)* ) => {
		java_like_enum!(
			@inner
			$visibility enum $name ;
			$($entryname1),* ;
			$(
				$attribname : $attribtype = {
					$(
						$entryname2 = $fieldvalue
					),*
				}
			),*
		);
	};

	(@recurse2 $visibility: vis enum $name: ident ; $($entryname1: ident),* ; $($attribname: ident : $attribtype: ty),* ; $( ($a1:ident, $b1:expr), $(($a: ident, $b: expr),)* ;)* ! $($(($a_out: ident, $b_out: expr),)* ;)* ) => {
		java_like_enum!(
			@recurse2
			$visibility enum $name ;
			$($entryname1),* ;
			$($attribname : $attribtype),* ;
			$($(($a, $b),)* ;)* !
			$($(($a_out, $b_out),)* ;)* $(($a1, $b1),)* ;
		);
	};

	(@inner $visibility: vis enum $name:ident ; $($entryname1: ident),* ; $($attribname:ident : $attribtype:ty = { $($entryname2:ident = $fieldvalue:expr),* } ),* ) => {
		#[derive(strum_macros::EnumIter, Copy, Clone)]
		$visibility enum $name {
			$($entryname1),*
		}
		impl $name {
			$(
				pub fn $attribname(self) -> $attribtype {
					match self {
						$(
							Self::$entryname2 => $fieldvalue
						),*
					}
				}
			)*
		}
	}
}
