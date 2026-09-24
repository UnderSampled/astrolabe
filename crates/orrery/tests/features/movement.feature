@movement @functional
Feature: Moving through the manor
  Background:
    Given Hype is ready to move in a clear part of the manor

  Scenario: Turning while standing
    When the player turns and releases the turning control
    Then Hype faces the new direction without walking away
    And Hype returns to standing without continuing to drift

  Scenario: Walking and stopping
    Given a clear path is ahead of Hype
    When the player walks forward and releases the movement control
    Then Hype moves forward from the starting place
    And Hype returns to standing without continuing to drift

  Scenario: Running instead of walking
    Given a clear path is ahead of Hype
    When the player walks and runs from equivalent starts
    Then running moves Hype faster than walking
