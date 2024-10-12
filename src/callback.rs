use std::str::FromStr;
pub enum CallbackCommand {
    Unsubscribe(u64),
    Update(u64),
    UpdateNickname(u64),
    UpdateFilter(u64),
    TrackBalance(u64),
    TrackReceive(u64),
    TrackSent(u64),
    TrackFull(u64),
}

#[derive(Debug, derive_more::Error, derive_more::Display)]
pub enum ParseCallbackErr {
    NoTypeProvided,
    TypeNotFound,
    ArgNotFound,
    IdParseError,
}
const UNSUB_INDEX: &str = "0";
const UPDATE_INDEX: &str = "1";
const NICKNAME_INDEX: &str = "2";
const FILTER_INDEX: &str = "3";
const BALANCE_INDEX: &str = "4";
const RECEIVE_INDEX: &str = "5";
const SENT_INDEX: &str = "6";
const FULL_INDEX: &str = "7";
impl CallbackCommand {
    fn index(&self) -> &str {
        match self {
            CallbackCommand::Unsubscribe(..) => UNSUB_INDEX,
            CallbackCommand::Update(..) => UPDATE_INDEX,
            CallbackCommand::UpdateNickname(..) => NICKNAME_INDEX,
            CallbackCommand::UpdateFilter(..) => FILTER_INDEX,
            CallbackCommand::TrackBalance(..) => BALANCE_INDEX,
            CallbackCommand::TrackReceive(..) => RECEIVE_INDEX,
            CallbackCommand::TrackSent(..) => SENT_INDEX,
            CallbackCommand::TrackFull(..) => FULL_INDEX,
        }
    }
    pub fn to_callback_data(&self) -> String {
        let index = self.index();
        match self {
            CallbackCommand::Unsubscribe(id) => {
                format!("{index} {id}")
            }
            CallbackCommand::Update(id) => {
                format!("{index} {id}")
            }
            CallbackCommand::UpdateNickname(id) => {
                format!("{index} {id}")
            }
            CallbackCommand::UpdateFilter(id) => {
                format!("{index} {id}")
            }
            CallbackCommand::TrackBalance(id) => {
                format!("{index} {id}")
            }
            CallbackCommand::TrackSent(id) => {
                format!("{index} {id}")
            }
            CallbackCommand::TrackReceive(id) => {
                format!("{index} {id}")
            }
            CallbackCommand::TrackFull(id) => {
                format!("{index} {id}")
            }
        }
    }
    pub fn from_string(data: String) -> Result<CallbackCommand, ParseCallbackErr> {
        let mut data = data.split_whitespace();
        match data.next() {
            Some(s) => match s {
                UNSUB_INDEX => match data.next() {
                    Some(id) => Ok(CallbackCommand::Unsubscribe(
                        u64::from_str(id).map_err(|_| ParseCallbackErr::IdParseError)?,
                    )),
                    None => Err(ParseCallbackErr::ArgNotFound),
                },
                UPDATE_INDEX => match data.next() {
                    Some(id) => Ok(CallbackCommand::Update(
                        u64::from_str(id).map_err(|_| ParseCallbackErr::IdParseError)?,
                    )),
                    None => Err(ParseCallbackErr::ArgNotFound),
                },
                NICKNAME_INDEX => match data.next() {
                    Some(id) => Ok(CallbackCommand::UpdateNickname(
                        u64::from_str(id).map_err(|_| ParseCallbackErr::IdParseError)?,
                    )),
                    None => Err(ParseCallbackErr::ArgNotFound),
                },
                FILTER_INDEX => match data.next() {
                    Some(id) => Ok(CallbackCommand::UpdateFilter(
                        u64::from_str(id).map_err(|_| ParseCallbackErr::IdParseError)?,
                    )),
                    None => Err(ParseCallbackErr::ArgNotFound),
                },
                BALANCE_INDEX => match data.next() {
                    Some(id) => Ok(CallbackCommand::TrackBalance(
                        u64::from_str(id).map_err(|_| ParseCallbackErr::IdParseError)?,
                    )),
                    None => Err(ParseCallbackErr::ArgNotFound),
                },
                RECEIVE_INDEX => match data.next() {
                    Some(id) => Ok(CallbackCommand::TrackReceive(
                        u64::from_str(id).map_err(|_| ParseCallbackErr::IdParseError)?,
                    )),
                    None => Err(ParseCallbackErr::ArgNotFound),
                },
                SENT_INDEX => match data.next() {
                    Some(id) => Ok(CallbackCommand::TrackSent(
                        u64::from_str(id).map_err(|_| ParseCallbackErr::IdParseError)?,
                    )),
                    None => Err(ParseCallbackErr::ArgNotFound),
                },
                FULL_INDEX => match data.next() {
                    Some(id) => Ok(CallbackCommand::TrackFull(
                        u64::from_str(id).map_err(|_| ParseCallbackErr::IdParseError)?,
                    )),
                    None => Err(ParseCallbackErr::ArgNotFound),
                },
                _ => Err(ParseCallbackErr::TypeNotFound),
            },
            None => Err(ParseCallbackErr::NoTypeProvided),
        }
    }
}
