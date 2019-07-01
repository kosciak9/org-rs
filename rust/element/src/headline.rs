//    This file is part of org-rs.
//
//    org-rs is free software: you can redistribute it and/or modify
//    it under the terms of the GNU General Public License as published by
//    the Free Software Foundation, either version 3 of the License, or
//    (at your option) any later version.
//
//    org-rs is distributed in the hope that it will be useful,
//    but WITHOUT ANY WARRANTY; without even the implied warranty of
//    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//    GNU General Public License for more details.
//
//    You should have received a copy of the GNU General Public License
//    along with org-rs.  If not, see <https://www.gnu.org/licenses/>.

//! Headlines and Sections
//! https://orgmode.org/worg/dev/org-syntax.html#Headlines_and_Sections
//! A headline is defined as:
//!
//! STARS KEYWORD PRIORITY TITLE TAGS
//!
//! STARS is a string starting at column 0, containing at least one asterisk (and up to
//! org-inlinetask-min-level if org-inlinetask library is loaded) and ended by a space character. The
//! number of asterisks is used to define the level of the headline. It’s the sole compulsory part of
//! a headline.
//!
//! KEYWORD is a TODO keyword, which has to belong to the list defined in org-todo-keywords-1. Case is
//! significant.
//!
//! PRIORITY is a priority cookie, i.e. a single letter preceded by a hash sign # and enclosed within
//! square brackets.
//!
//! TITLE can be made of any character but a new line. Though, it will match after every other part
//! have been matched.
//!
//! TAGS is made of words containing any alpha-numeric character, underscore, at sign, hash sign or
//! percent sign, and separated with colons.
//!
//! Examples of valid headlines include:
//!
//!
//! *
//!
//! ** DONE
//!
//! *** Some e-mail
//!
//! **** TODO [#A] COMMENT Title :tag:a2%:
//!
//!
//! If the first word appearing in the title is “COMMENT”, the headline will be considered as
//! “commented”. Case is significant.
//!
//! If its title is org-footnote-section, it will be considered as a “footnote section”. Case is
//! significant.
//!
//! If “ARCHIVE” is one of its tags, it will be considered as “archived”. Case is significant.
//!
//! A headline contains directly one section (optionally), followed by any number of deeper level
//! headlines.
//!
//! A section contains directly any greater element or element. Only a headline can contain a section.
//! As an exception, text before the first headline in the document also belongs to a section.
//!
//! As an example, consider the following document:
//!
//! An introduction.
//!
//! * A Headline
//!
//! Some text.
//!
//! ** Sub-Topic 1
//!
//! ** Sub-Topic 2
//!
//! *** Additional entry
//!
//! Its internal structure could be summarized as:
//!
//! (document
//!  (section)
//!  (headline
//!   (section)
//!   (headline)
//!   (headline
//!    (headline))))
//!

use crate::data::{SyntaxNode, TimestampData};
use crate::parser::Parser;
use regex::Regex;
use std::borrow::Cow;

const ORG_CLOSED_STRING: &str = "CLOSED";
const ORG_DEADLINE_STRING: &str = "DEADLINE";
const ORG_SCHEDULED_STRING: &str = "SCHEDULED";

lazy_static! {
    pub static ref REGEX_HEADLINE_SHORT: Regex = Regex::new(r"^\*+\s").unwrap();

    // TODO document why is it needed and what are the consequences of using multiline regex
    pub static ref REGEX_HEADLINE_MULTILINE: Regex = Regex::new(r"(?m)^\*+\s").unwrap();

    /// Matches a line with planning info.
    /// Matched keyword is in group 1
    pub static ref REGEX_PLANNING_LINE: Regex = Regex::new(
        &format!(r"^[ \t]*((?:{}|{}|{}):)",
            ORG_CLOSED_STRING, ORG_DEADLINE_STRING, ORG_SCHEDULED_STRING ))
        .unwrap();

    /// Matches an entire property drawer
    /// Requires multiline match
    /// correspond to org-property-drawer-re in org.el
    pub static ref REGEX_PROPERTY_DRAWER: Regex = Regex::new(
        r"(?i)^[ \t]*:PROPERTIES:[ \t]*\n(?:[ \t]*:\S+:(?: .*)?[ \t]*\n)*?[ \t]*:END:[ \t]*")
            .unwrap();

    pub static ref REGEX_CLOCK_LINE: Regex = Regex::new(r"(?i)^[ \t]*CLOCK:").unwrap();

    /// Matches any of the TODO state keywords.
    /// TODO parametrize
    pub static ref REGEX_TODO: Regex = Regex::new(r"(?i)(TODO|DONE)[ \t]").unwrap();

    
    /// TODO parametrize
    /// check how org-done-keywords are set
    pub static ref REGEX_TODO_DONE: Regex = Regex::new(r"(?i)DONE").unwrap();


    pub static ref REGEX_HEADLINE_PRIORITY: Regex = Regex::new(r"\[#.\][ \t]*").unwrap();


}

pub struct HeadlineData<'a> {
    /// Non_nil if the headline has an archive tag (boolean).
    archivedp: bool,

    /// Headline's CLOSED reference, if any (timestamp object or nil)
    closed: Option<TimestampData<'a>>,

    /// Non_nil if the headline has a comment keyword (boolean).
    commentedp: bool,

    /// Headline's DEADLINE reference, if any (timestamp object or nil).
    deadline: Option<TimestampData<'a>>,

    /// Non_nil if the headline is a footnote section (boolean).
    footnote_section_p: bool,

    /// Reduced level of the headline (integer).
    level: usize,

    /// Number of blank lines between the headline
    /// and the first non_blank line of its contents (integer).
    pre_blank: usize,

    /// Headline's priority, as a character (integer).
    priority: Option<usize>,

    /// Non_nil if the headline contains a quote keyword (boolean).
    quotedp: bool,

    /// Raw headline's text, without the stars and the tags (string).
    raw_value: Cow<'a, str>,

    /// Headline's SCHEDULED reference, if any (timestamp object or nil).
    scheduled: Option<TimestampData<'a>>,

    /// Headline's tags, if any, without
    /// the archive tag. (list of strings).
    tags: Vec<Tag<'a>>,

    /// Parsed headline's text, without the stars
    /// and the tags (secondary string).
    title: Option<Cow<'a, str>>,

    /// Headline's TODO keyword without quote and comment
    /// strings, if any (string or nil).
    /// also used instead of todo-type
    todo_keyword: Option<TodoKeyword<'a>>,
}

// A planning is an element with the following pattern:
// HEADLINE
// PLANNING
//
// where HEADLINE is a headline element and PLANNING is a line filled with INFO parts, where each of them follows the pattern:
//
// KEYWORD: TIMESTAMP
//
// KEYWORD is either “DEADLINE”, “SCHEDULED” or “CLOSED”. TIMESTAMP is a timestamp object.
//
// In particular, no blank line is allowed between PLANNING and HEADLINE.

pub struct NodePropertyData<'a> {
    key: Cow<'a, str>,
    value: Cow<'a, str>,
}

pub struct Tag<'a>(Cow<'a, str>);

pub struct TodoKeyword<'a>(Cow<'a, str>);


// TODO this have to be defined by user set vaiable
impl<'a> TodoKeyword<'a> {
    fn is_done(&self) -> bool {
        REGEX_TODO_DONE.find(&self.0).is_some()
    }

}

pub enum TodoType {
    TODO,
    DONE,
}

impl<'a> Parser<'a> {

    /// Parse a headline.
    /// Return a list whose CAR is `headline' and CDR is a plist
    /// containing `:raw-value', `:title', `:begin', `:end',
    /// `:pre-blank', `:contents-begin' and `:contents-end', `:level',
    /// `:priority', `:tags', `:todo-keyword',`:todo-type', `:scheduled',
    /// `:deadline', `:closed', `:archivedp', `:commentedp'
    /// `:footnote-section-p', `:post-blank' and `:post-affiliated'
    /// keywords.
    ///
    /// The plist also contains any property set in the property drawer,
    /// with its name in upper cases and colons added at the
    /// beginning (e.g., `:CUSTOM_ID').
    ///
    /// LIMIT is a buffer position bounding the search.
    ///
    /// When RAW-SECONDARY-P is non-nil, headline's title will not be
    /// parsed as a secondary string, but as a plain string instead.
    ///
    /// Assume point is at beginning of the headline."

    pub fn headline_parser(&self, limit: usize, raw_secondary_p: bool) -> SyntaxNode<'a> {
        let mut cursor = self.cursor.borrow_mut();
        let begin = cursor.pos();

        let level = cursor.skip_chars_forward("*", Some(limit));
        cursor.skip_chars_forward(" \t", Some(limit));

        let todo = match cursor.capturing_at(&*REGEX_TODO) {
            None => None,
            Some(m) => {
                let m0 = m.get(0).unwrap();
                let m1 = m.get(1).unwrap();
                cursor.set(m0.end());
                cursor.skip_chars_forward(" \t", Some(limit));
                Some(Cow::from(&self.input[m1.start()..m1.end()]))
            }
        };

        // todo_type was moved into a method

        let priority = match cursor.looking_at(&*REGEX_HEADLINE_PRIORITY) {
            None => None,
            Some(m) => {
                cursor.set(m.end());
                //FIXME integer??
                Some()
            }

        }

        // 	   (priority (and (looking-at "\\[#.\\][ \t]*")
        // 			  (progn (goto-char (match-end 0))
        // 				 (aref (match-string 0) 2))))


        cursor.set(begin);
        unimplemented!()
        //   (save-excursion
        //     (let* ((begin (point))
        // 	   (level (prog1 (org-reduced-level (skip-chars-forward "*"))
        // 		    (skip-chars-forward " \t")))
        // 	   (todo (and org-todo-regexp
        // 		      (let (case-fold-search) (looking-at (concat org-todo-regexp " ")))
        // 		      (progn (goto-char (match-end 0))
        // 			     (skip-chars-forward " \t")
        // 			     (match-string 1))))
        // 	   (todo-type
        // 	    (and todo (if (member todo org-done-keywords) 'done 'todo)))
        // 	   (priority (and (looking-at "\\[#.\\][ \t]*")
        // 			  (progn (goto-char (match-end 0))
        // 				 (aref (match-string 0) 2))))
        // 	   (commentedp
        // 	    (and (let (case-fold-search) (looking-at org-comment-string))
        // 		 (goto-char (match-end 0))))
        // 	   (title-start (point))
        // 	   (tags (when (re-search-forward
        // 			"[ \t]+\\(:[[:alnum:]_@#%:]+:\\)[ \t]*$"
        // 			(line-end-position)
        // 			'move)
        // 		   (goto-char (match-beginning 0))
        // 		   (org-split-string (match-string 1) ":")))
        // 	   (title-end (point))
        // 	   (raw-value (org-trim
        // 		       (buffer-substring-no-properties title-start title-end)))
        // 	   (archivedp (member org-archive-tag tags))
        // 	   (footnote-section-p (and org-footnote-section
        // 				    (string= org-footnote-section raw-value)))
        // 	   (standard-props (org-element--get-node-properties))
        // 	   (time-props (org-element--get-time-properties))
        // 	   (end (min (save-excursion (org-end-of-subtree t t)) limit))
        // 	   (contents-begin (save-excursion
        // 			     (forward-line)
        // 			     (skip-chars-forward " \r\t\n" end)
        // 			     (and (/= (point) end) (line-beginning-position))))
        // 	   (contents-end (and contents-begin
        // 			      (progn (goto-char end)
        // 				     (skip-chars-backward " \r\t\n")
        // 				     (line-beginning-position 2)))))
        //       (let ((headline
        // 	     (list 'headline
        // 		   (nconc
        // 		    (list :raw-value raw-value
        // 			  :begin begin
        // 			  :end end
        // 			  :pre-blank
        // 			  (if (not contents-begin) 0
        // 			    (1- (count-lines begin contents-begin)))
        // 			  :contents-begin contents-begin
        // 			  :contents-end contents-end
        // 			  :level level
        // 			  :priority priority
        // 			  :tags tags
        // 			  :todo-keyword todo
        // 			  :todo-type todo-type
        // 			  :post-blank
        // 			  (if contents-end
        // 			      (count-lines contents-end end)
        // 			    (1- (count-lines begin end)))
        // 			  :footnote-section-p footnote-section-p
        // 			  :archivedp archivedp
        // 			  :commentedp commentedp
        // 			  :post-affiliated begin)
        // 		    time-props
        // 		    standard-props))))
        // 	(org-element-put-property
        // 	 headline :title
        // 	 (if raw-secondary-p raw-value
        // 	   (org-element--parse-objects
        // 	    (progn (goto-char title-start)
        // 		   (skip-chars-forward " \t")
        // 		   (point))
        // 	    (progn (goto-char title-end)
        // 		   (skip-chars-backward " \t")
        // 		   (point))
        // 	    nil
        // 	    (org-element-restriction 'headline)
        // 	    headline)))))))
        //
    }

    // TODO implement inlinetask_parser
    pub fn inlinetask_parser(&self, limit: usize, raw_secondary_p: bool) -> SyntaxNode<'a> {
        unimplemented!()
    }

    // TODO implement property_drawer_parser
    pub fn property_drawer_parser(&self, limit: usize) -> SyntaxNode<'a> {
        unimplemented!()
    }

    // TODO implement node_property_parser
    pub fn node_property_parser(&self, limit: usize) -> SyntaxNode<'a> {
        unimplemented!()
    }
}
