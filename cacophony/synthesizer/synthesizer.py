from abc import ABC, abstractmethod
from overrides import final
from cacophony.music.note import Note


class Synthesizer(ABC):
    @final
    def audio(self, note: Note, bpm: int) -> bytes:
        """
        Synthesize a note.

        :param note: The note.
        :param bpm: The beats per minute.

        :return: A bytestring of audio samples.
        """

        # Return silence.
        if note.note is None:
            return b''
        # Return a note.
        else:
            return self._audio(note=note, duration=Synthesizer._get_duration(bpm=bpm, beat=note.duration))

    @abstractmethod
    def _audio(self, note: Note, duration: float) -> bytes:
        """
        Synthesize a note.

        :param note: The note.
        :param duration: The duration of the note in seconds.

        :return: A bytestring of audio samples.
        """

        raise Exception()

    @staticmethod
    def _get_duration(bpm: int, beat: float) -> float:
        """
        :param bpm: The beats per minute.
        :param beat: The duration in terms of beats.

        :return: The duration in terms of seconds.
        """

        return 60.0 / bpm * beat
