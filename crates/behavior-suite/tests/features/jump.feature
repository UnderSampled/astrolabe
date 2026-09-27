@jump @functional
Feature: Jumping in the manor
  Background:
    Given Hype is ready to move in a clear part of the manor

  Scenario: Jumping from a standstill and landing
    Given Hype has backed into a clear grounded jump start
    When the player presses and releases jump while standing
    Then Hype rises above the floor during the jump
    And Hype returns to the starting floor and settles

  Scenario: Jumping from a walk and landing
    Given a clear path is ahead of Hype
    When the player jumps while continuously walking forward
    Then Hype jumps upward and forward from the walk
    And Hype lands and settles after the moving jump

  Scenario: A moving jump meets the wall and lands
    Given Hype is grounded near the verified manor corner wall
    When the player jumps forward toward the wall
    Then Hype meets the wall while airborne without passing through
    And Hype lands and settles after meeting the wall
